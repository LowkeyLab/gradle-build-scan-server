extern crate format as proxy_format;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::response::Response;
use axum::{Extension, Router};
use base64::Engine as _;
use chrono::Utc;
use tokio::signal;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

use config::Config;
use proxy_format::{Payload, RequestData, ResponseData};
use service::ProxyService;

const MAX_BODY_SIZE: usize = 10 * 1024 * 1024; // 10 MB

#[derive(Clone)]
struct AppState {
    config: Config,
    client: reqwest::Client,
    service: Arc<ProxyService>,
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

    // Connect to database
    let pool = db::connect(&config.database_url)
        .await
        .expect("Failed to connect to database");
    info!("Connected to database: {}", config.database_url);

    let service = Arc::new(ProxyService::new(pool));

    // GraphQL
    let schema = Arc::new(graphql::create_schema());
    let gql_context = Arc::new(graphql::Context {
        service: service.clone(),
    });

    let client = reqwest::Client::builder()
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .connect_timeout(std::time::Duration::from_secs(30))
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .expect("Failed to create HTTP client");

    let state = AppState {
        config: config.clone(),
        client,
        service,
    };

    let app = Router::new()
        .route(
            "/graphql",
            axum::routing::get(graphql::graphql_handler).post(graphql::graphql_handler),
        )
        .route(
            "/graphiql",
            axum::routing::get(juniper_axum::graphiql("/graphql", None::<&str>)),
        )
        .fallback(proxy_handler)
        .layer(Extension(schema))
        .layer(Extension(gql_context))
        .with_state(state);

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind to port {}: {}", config.port, e);
            std::process::exit(1);
        }
    };
    info!(
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

    info!("Shutting down server...");
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
            error!("Failed to read request body: {}", e);
            return Response::builder()
                .status(413)
                .body(Body::from("Payload too large"))
                .unwrap_or_else(|_| Response::new(Body::from("Payload too large")));
        }
    };
    let request_body = match String::from_utf8(body_bytes.to_vec()) {
        Ok(s) => serde_json::json!(s),
        Err(_) => serde_json::json!({
            "base64": base64::engine::general_purpose::STANDARD.encode(&body_bytes)
        }),
    };

    let request_id = Uuid::new_v4().to_string();
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S%.3f").to_string();

    // Build upstream URL
    let upstream_url = format!("{}{}", state.config.upstream_url, path_and_query);

    // Build upstream request, forwarding non-hop-by-hop headers
    let mut upstream_headers = reqwest::header::HeaderMap::new();
    for (name, value) in &request_headers {
        if !is_hop_by_hop(name)
            && let (Ok(hn), Ok(hv)) = (
                reqwest::header::HeaderName::from_bytes(name.as_bytes()),
                reqwest::header::HeaderValue::from_str(value),
            )
        {
            upstream_headers.insert(hn, hv);
        }
    }

    // Set Host header to the upstream host
    if let Ok(upstream) = reqwest::Url::parse(&upstream_url)
        && let Some(host) = upstream.host_str()
    {
        let host_value = match upstream.port() {
            Some(p) => format!("{}:{}", host, p),
            None => host.to_string(),
        };
        if let Ok(hv) = reqwest::header::HeaderValue::from_str(&host_value) {
            upstream_headers.insert(reqwest::header::HOST, hv);
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
            let response_body = match String::from_utf8(response_body_bytes.to_vec()) {
                Ok(s) => serde_json::json!(s),
                Err(_) => serde_json::json!({
                    "base64": base64::engine::general_purpose::STANDARD.encode(&response_body_bytes)
                }),
            };

            let response_data = ResponseData {
                status: Some(status),
                headers: Some(
                    response_headers
                        .iter()
                        .filter(|(k, _)| !is_hop_by_hop(k))
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<Vec<_>>(),
                ),
                body: Some(response_body),
                error: None,
            };

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
            error!("Upstream request failed: {}", e);
            let response_data = ResponseData {
                error: Some(e.to_string()),
                status: None,
                headers: None,
                body: None,
            };
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

    // Store payload to database (best-effort)
    let payload = Payload {
        request_id: request_id.clone(),
        timestamp: timestamp.clone(),
        request: RequestData {
            method: method.to_string(),
            uri: path_and_query.to_string(),
            headers: request_headers,
            body: request_body,
        },
        response: response_data,
    };

    if let Err(e) = state.service.store_payload(&payload).await {
        error!("Failed to store payload: {}", e);
    } else {
        info!(
            "Stored payload {} for {} {}",
            request_id, method, path_and_query
        );
    }

    http_response
}
