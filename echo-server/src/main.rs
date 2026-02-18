use axum::{
    body::Body,
    extract::Request,
    response::Response,
    routing::any,
    Router,
};
use chrono::Utc;
use std::net::SocketAddr;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let app = Router::new().route("/", any(echo_handler));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Echo server listening on http://{}", addr);

    axum::serve(listener, app).await.unwrap();
}

async fn echo_handler(request: Request<Body>) -> Response<Body> {
    let method = request.method().clone();
    let uri = request.uri().clone();
    
    let headers: Vec<_> = request
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let body_bytes = axum::body::to_bytes(request.into_body(), usize::MAX).await.unwrap();
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

    let payload_str = serde_json::to_string_pretty(&payload).unwrap();
    
    let dir = std::path::Path::new("/tmp/gradle-payloads");
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join(&filename);
    std::fs::write(&path, &payload_str).unwrap();
    
    println!("Saved payload to: {:?}", path);

    Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(body_str.to_string()))
        .unwrap()
}
