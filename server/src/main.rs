use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::Extension;
use axum::Router;
use axum::routing::MethodFilter;
use axum::routing::{get, on, post};
use juniper_axum::{extract::JuniperRequest, graphiql, response::JuniperResponse};
use sqlx::SqlitePool;
use tokio::signal;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use config::Config;
use ingest::{IngestAppState, IngestState, UploadedScan};
use models::TaskOutcome;

async fn graphql_handler(
    Extension(schema): Extension<Arc<graphql::Schema>>,
    Extension(context): Extension<Arc<graphql::Context>>,
    JuniperRequest(request): JuniperRequest,
) -> JuniperResponse {
    JuniperResponse(request.execute(&*schema, &*context).await)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env();
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

    let pool = match db::connect(&config.db_url).await {
        Ok(p) => {
            info!("Connected to database");
            p
        }
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            std::process::exit(1);
        }
    };

    let base_url = config.base_url.clone();

    let on_upload = {
        let pool = pool.clone();
        Arc::new(move |uploaded: UploadedScan| {
            let pool = pool.clone();
            tokio::spawn(async move {
                process_upload(pool, uploaded).await;
            });
        })
    };

    let schema = Arc::new(graphql::create_schema());
    let context = Arc::new(graphql::Context {
        pool: Arc::new(pool.clone()),
    });

    let ingest_state = IngestAppState {
        base_url,
        ingest: IngestState::new(),
        on_upload,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/scans/publish", post(ingest::handle_token_request))
        .route(
            "/scans/publish/{id}/upload",
            post(ingest::handle_scan_upload),
        )
        .with_state(ingest_state)
        .route(
            "/graphql",
            on(MethodFilter::GET.or(MethodFilter::POST), graphql_handler),
        )
        .route("/graphiql", get(graphiql("/graphql", None::<&str>)))
        .layer(Extension(schema))
        .layer(Extension(context))
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024))
        .layer(cors);

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind to port {}: {}", config.port, e);
            std::process::exit(1);
        }
    };
    info!("Build scan server listening on http://{}", addr);
    info!("GraphiQL IDE available at http://{}/graphiql", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Server error");
}

async fn process_upload(pool: SqlitePool, uploaded: UploadedScan) {
    let scan_id = uploaded.scan_id.clone();
    let payload_size = uploaded.raw_payload.len();
    info!(scan_id = %scan_id, payload_size = payload_size, "Received build scan upload");

    match lib::parse(&uploaded.raw_payload) {
        Err(e) => {
            error!(scan_id = %scan_id, error = %e, "Failed to parse build scan payload");
            if let Err(db_err) = db::insert_build_scan(
                &pool,
                &scan_id,
                uploaded.build_tool_type.as_deref().unwrap_or(""),
                uploaded.build_tool_version.as_deref().unwrap_or(""),
                uploaded.plugin_version.as_deref().unwrap_or(""),
                None,
                None,
                Some("parse_error"),
                None,
                None,
                None,
                None,
                None,
                None,
                Some(&uploaded.raw_payload),
            )
            .await
            {
                error!(scan_id = %scan_id, error = %db_err, "Failed to store parse_error scan");
            }
        }
        Ok(payload) => {
            let outcome = if payload
                .tasks
                .iter()
                .any(|t| matches!(t.outcome, Some(TaskOutcome::Failed)))
            {
                "failed"
            } else {
                "success"
            };

            let requested_tasks_json = if payload.requested_tasks.is_empty() {
                None
            } else {
                serde_json::to_string(&payload.requested_tasks).ok()
            };

            if let Err(db_err) = db::insert_build_scan(
                &pool,
                &scan_id,
                uploaded.build_tool_type.as_deref().unwrap_or(""),
                uploaded.build_tool_version.as_deref().unwrap_or(""),
                uploaded.plugin_version.as_deref().unwrap_or(""),
                None,
                None,
                Some(outcome),
                requested_tasks_json.as_deref(),
                payload.hostname.as_deref(),
                payload.os_name.as_deref(),
                payload.os_version.as_deref(),
                payload.jvm_vendor.as_deref(),
                payload.jvm_version.as_deref(),
                Some(&uploaded.raw_payload),
            )
            .await
            {
                error!(scan_id = %scan_id, error = %db_err, "Failed to store build scan");
                return;
            }

            let task_count = payload.tasks.len();
            for task in &payload.tasks {
                let outcome_str = task.outcome.as_ref().map(|o| format!("{:?}", o));
                let cache_key_str = task.origin_build_cache_key.as_ref().map(|k| hex::encode(k));

                if let Err(db_err) = db::insert_task(
                    &pool,
                    &scan_id,
                    &task.task_path,
                    task.class_name.as_deref(),
                    outcome_str.as_deref(),
                    task.cacheable,
                    task.started_at,
                    task.finished_at,
                    cache_key_str.as_deref(),
                    None,
                )
                .await
                {
                    error!(scan_id = %scan_id, task_path = %task.task_path, error = %db_err, "Failed to store task");
                }
            }

            info!(scan_id = %scan_id, task_count = task_count, "Stored build scan successfully");
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutting down server...");
}
