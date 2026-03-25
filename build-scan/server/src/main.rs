use std::net::SocketAddr;
use std::sync::Arc;

use axum::Extension;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::MethodFilter;
use axum::routing::{get, on, post};
use juniper_axum::graphiql;
use tokio::signal;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use config::Config;
use ingest::{IngestAppState, IngestState};
use service::BuildScanService;

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

    let service = Arc::new(BuildScanService::new(pool.clone()));

    let schema = Arc::new(graphql::create_schema());
    let context = Arc::new(graphql::Context {
        service: service.clone(),
    });

    let ingest_state = IngestAppState {
        base_url: config.base_url.clone(),
        ingest: IngestState::new(),
        service,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let mut app = Router::new()
        .route(
            "/scans/publish/{tool_type}/{version}/token",
            post(ingest::handle_token_request),
        )
        .route(
            "/scans/publish/{tool_type}/{version}/upload",
            post(ingest::handle_scan_upload),
        )
        .route("/usage/users/check", get(ingest::handle_usage_check))
        .with_state(ingest_state)
        .route(
            "/graphql",
            on(
                MethodFilter::GET.or(MethodFilter::POST),
                graphql::graphql_handler,
            ),
        )
        .route(
            "/web/graphql",
            on(
                MethodFilter::GET.or(MethodFilter::POST),
                graphql::graphql_handler,
            ),
        )
        .route("/graphiql", get(graphiql("/graphql", None::<&str>)))
        .layer(Extension(schema))
        .layer(Extension(context))
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024))
        .layer(cors);

    if let Some(ref spa_dir) = config.spa_dir {
        app = app.nest_service(
            "/web",
            ServeDir::new(spa_dir.as_path()).fallback(ServeFile::new(spa_dir.join("index.html"))),
        );
    }

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind to port {}: {}", config.port, e);
            std::process::exit(1);
        }
    };
    let local_addr = listener.local_addr().expect("Failed to get local address");
    info!("Build scan server listening on http://{local_addr}");
    info!("  Local:    http://localhost:{}/", local_addr.port());
    info!(
        "  GraphiQL: http://localhost:{}/graphiql",
        local_addr.port()
    );
    if config.spa_dir.is_some() {
        info!("  Web UI:   http://localhost:{}/web/", local_addr.port());
    }

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Server error");
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
