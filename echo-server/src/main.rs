use axum::{Router, body::Body, extract::Request, extract::State, response::Response};
use chrono::Utc;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::signal;
use uuid::Uuid;

const MAX_BODY_SIZE: usize = 10 * 1024 * 1024; // 10 MB

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

const HOP_BY_HOP_HEADERS: &[&str] = &[
    "connection",
    "host",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

fn is_hop_by_hop(name: &str) -> bool {
    HOP_BY_HOP_HEADERS
        .iter()
        .any(|h| h.eq_ignore_ascii_case(name))
}

async fn proxy_handler(State(state): State<AppState>, request: Request<Body>) -> Response<Body> {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path_and_query = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");

    let request_headers: Vec<_> = request
        .headers()
        .iter()
        .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
        .collect();

    // Read request body
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

    let request_id = Uuid::new_v4().to_string();
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S%.3f").to_string();

    // Build upstream URL
    let upstream_url = format!("{}{}", state.config.upstream_url, path_and_query);

    // Build upstream request, forwarding non-hop-by-hop headers
    let mut upstream_headers = reqwest::header::HeaderMap::new();
    for (name, value) in &request_headers {
        if !is_hop_by_hop(name) {
            if let (Ok(hn), Ok(hv)) = (
                reqwest::header::HeaderName::from_bytes(name.as_bytes()),
                reqwest::header::HeaderValue::from_str(value),
            ) {
                upstream_headers.insert(hn, hv);
            }
        }
    }

    // Set Host header to the upstream host
    if let Ok(upstream) = reqwest::Url::parse(&upstream_url) {
        if let Some(host) = upstream.host_str() {
            let host_value = match upstream.port() {
                Some(p) => format!("{}:{}", host, p),
                None => host.to_string(),
            };
            if let Ok(hv) = reqwest::header::HeaderValue::from_str(&host_value) {
                upstream_headers.insert(reqwest::header::HOST, hv);
            }
        }
    }

    // Forward request upstream
    let upstream_result = state
        .client
        .request(
            reqwest::Method::from_bytes(method.as_str().as_bytes())
                .expect("HTTP method should always be valid"),
            &upstream_url,
        )
        .headers(upstream_headers)
        .body(body_bytes.to_vec())
        .send()
        .await;

    // Build payload and response
    let (response_data, http_response) = match upstream_result {
        Ok(upstream_response) => {
            let status = upstream_response.status().as_u16();
            let response_headers: Vec<_> = upstream_response
                .headers()
                .iter()
                .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
                .collect();

            let response_body_bytes = upstream_response.bytes().await.unwrap_or_default();
            let response_body_str = String::from_utf8_lossy(&response_body_bytes);

            let response_data = serde_json::json!({
                "status": status,
                "headers": response_headers.iter()
                    .filter(|(k, _)| !is_hop_by_hop(k))
                    .collect::<Vec<_>>(),
                "body": response_body_str,
            });

            // Build HTTP response to return to client
            let mut builder = Response::builder().status(status);
            for (name, value) in &response_headers {
                if !is_hop_by_hop(name) {
                    builder = builder.header(name.as_str(), value.as_str());
                }
            }
            let http_response = builder
                .body(Body::from(response_body_bytes))
                .unwrap_or_else(|_| {
                    Response::builder()
                        .status(500)
                        .body(Body::from("Failed to build response"))
                        .unwrap()
                });

            (response_data, http_response)
        }
        Err(e) => {
            eprintln!("Upstream request failed: {}", e);
            let response_data = serde_json::json!({
                "error": e.to_string(),
                "status": null,
            });
            let http_response = Response::builder()
                .status(502)
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"error": "Bad Gateway", "detail": e.to_string()})
                        .to_string(),
                ))
                .unwrap_or_else(|_| Response::new(Body::from("Bad Gateway")));

            (response_data, http_response)
        }
    };

    // Save payload (best-effort)
    let payload = serde_json::json!({
        "request_id": request_id,
        "timestamp": timestamp,
        "request": {
            "method": method.to_string(),
            "uri": path_and_query,
            "headers": request_headers,
            "body": body_str,
        },
        "response": response_data,
    });

    let dir = &state.config.payload_dir;
    if let Err(e) = tokio::fs::create_dir_all(dir).await {
        eprintln!("Failed to create directory {:?}: {}", dir, e);
    } else {
        let filename = format!("{}-{}.json", timestamp, request_id);
        let path = dir.join(&filename);
        match serde_json::to_string_pretty(&payload) {
            Ok(s) => {
                if let Err(e) = tokio::fs::write(&path, &s).await {
                    eprintln!("Failed to write payload: {}", e);
                } else {
                    println!("Saved payload to: {:?}", path);
                }
            }
            Err(e) => eprintln!("Failed to serialize payload: {}", e),
        }
    }

    http_response
}
