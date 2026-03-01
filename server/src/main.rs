use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::routing::post;
use tokio::signal;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use config::Config;
use ingest::{IngestAppState, IngestState, UploadedScan};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env();
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

    let base_url = format!("http://{}", addr);

    let on_upload = Arc::new(|uploaded: UploadedScan| {
        let scan_id = uploaded.scan_id.clone();
        let payload_size = uploaded.raw_payload.len();
        info!(scan_id = %scan_id, payload_size = payload_size, "Received build scan upload");
        match lib::parse(&uploaded.raw_payload) {
            Ok(payload) => {
                let task_count = payload.tasks.len();
                info!(scan_id = %scan_id, task_count = task_count, "Successfully parsed build scan");
            }
            Err(e) => {
                error!(scan_id = %scan_id, error = %e, "Failed to parse build scan payload");
            }
        }
    });

    let ingest_state = IngestAppState {
        base_url,
        ingest: IngestState::new(),
        on_upload,
    };

    let app = Router::new()
        .route("/scans/publish", post(ingest::handle_token_request))
        .route(
            "/scans/publish/{id}/upload",
            post(ingest::handle_scan_upload),
        )
        .with_state(ingest_state);

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind to port {}: {}", config.port, e);
            std::process::exit(1);
        }
    };
    info!("Build scan server listening on http://{}", addr);

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
