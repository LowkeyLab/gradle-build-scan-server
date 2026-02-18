use axum::{Router, body::Body, extract::Request, response::Response, routing::any};
use chrono::Utc;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::signal;
use uuid::Uuid;

const MAX_BODY_SIZE: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone)]
struct Config {
    port: u16,
    payload_dir: PathBuf,
}

impl Config {
    fn from_env() -> Self {
        Self {
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            payload_dir: std::env::var("PAYLOAD_DIR")
                .ok()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/tmp/gradle-payloads")),
        }
    }
}

#[tokio::main]
async fn main() {
    let config = Config::from_env();
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let app = Router::new().route("/", any(echo_handler));

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind to port {}: {}", config.port, e);
            std::process::exit(1);
        }
    };
    println!("Echo server listening on http://{}", addr);

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

async fn echo_handler(request: Request<Body>) -> Response<Body> {
    let method = request.method().clone();
    let uri = request.uri().clone();

    let headers: Vec<_> = request
        .headers()
        .iter()
        .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
        .collect();

    let body_bytes = match axum::body::to_bytes(request.into_body(), MAX_BODY_SIZE).await {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Failed to read request body: {}", e);
            return Response::builder()
                .status(413)
                .body(Body::from("Payload too large"))
                .unwrap_or_else(|_| Response::new(Body::from("Payload too large")));
        }
    };
    let body_str = String::from_utf8_lossy(&body_bytes);

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S%.3f").to_string();
    let uuid = Uuid::new_v4();
    let filename = format!("{}-{}.json", timestamp, uuid);

    let payload = serde_json::json!({
        "timestamp": timestamp,
        "method": method.to_string(),
        "uri": uri.to_string(),
        "headers": headers,
        "body": body_str,
    });

    let payload_str = match serde_json::to_string_pretty(&payload) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to serialize payload: {}", e);
            return Response::builder()
                .status(500)
                .body(Body::from("Internal server error"))
                .unwrap_or_else(|_| Response::new(Body::from("Internal server error")));
        }
    };

    let config = Config::from_env();
    let dir = config.payload_dir;

    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("Failed to create directory {:?}: {}", dir, e);
        return Response::builder()
            .status(500)
            .body(Body::from("Internal server error"))
            .unwrap_or_else(|_| Response::new(Body::from("Internal server error")));
    }

    let path = dir.join(&filename);
    if let Err(e) = std::fs::write(&path, &payload_str) {
        eprintln!("Failed to write payload: {}", e);
        return Response::builder()
            .status(500)
            .body(Body::from("Internal server error"))
            .unwrap_or_else(|_| Response::new(Body::from("Internal server error")));
    }

    println!("Saved payload to: {:?}", path);

    Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(body_str.to_string()))
        .unwrap_or_else(|_| Response::new(Body::from(body_str.to_string())))
}
