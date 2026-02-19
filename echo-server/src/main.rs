use axum::{Router, body::Body, extract::Request, extract::State, response::Response};
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::signal;

#[derive(Debug, Clone)]
struct Config {
    port: u16,
    payload_dir: PathBuf,
    upstream_url: String,
}

impl Config {
    fn from_env() -> Self {
        let upstream_url =
            std::env::var("UPSTREAM_URL").expect("UPSTREAM_URL environment variable is required");

        // Strip trailing slash for consistent URL joining
        let upstream_url = upstream_url.trim_end_matches('/').to_string();

        Self {
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            payload_dir: std::env::var("PAYLOAD_DIR")
                .ok()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/tmp/gradle-payloads")),
            upstream_url,
        }
    }
}

#[derive(Debug, Clone)]
struct AppState {
    config: Config,
    client: reqwest::Client,
}

#[tokio::main]
async fn main() {
    let config = Config::from_env();
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .expect("Failed to create HTTP client");

    let state = AppState {
        config: config.clone(),
        client,
    };

    let app = Router::new().fallback(proxy_handler).with_state(state);

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind to port {}: {}", config.port, e);
            std::process::exit(1);
        }
    };
    println!(
        "Proxy server listening on http://{}, forwarding to {}",
        addr, config.upstream_url
    );

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

    println!("Shutting down server...");
}

async fn proxy_handler(State(state): State<AppState>, request: Request<Body>) -> Response<Body> {
    // TODO: implement proxy logic in next task
    let _ = state;
    let _ = request;
    Response::builder()
        .status(501)
        .body(Body::from("Not implemented"))
        .unwrap()
}
