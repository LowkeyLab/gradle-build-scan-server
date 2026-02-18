# Echo Server Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** A simple HTTP echo server that saves all incoming payloads to disk while echoing responses back, used to reverse engineer Gradle develocity client payloads.

**Architecture:** Single Rust binary using axum framework. Server listens on port 8080, saves request payloads to `/tmp/gradle-payloads/`, and echoes the body back to the client.

**Tech Stack:** Rust, axum, tokio, serde, uuid, chrono

---

### Task 1: Create echo-server directory and Cargo.toml

**Files:**
- Create: `echo-server/Cargo.toml`
- Create: `echo-server/src/main.rs`

**Step 1: Create echo-server directory**

```bash
mkdir -p echo-server/src
```

**Step 2: Write Cargo.toml**

```toml
[package]
name = "echo-server"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "echo-server"
path = "src/main.rs"

[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
serde = "1"
serde_json = "1"
uuid = { version = "1", features = ["v4"] }
chrono = "0.4"
tower = { version = "0.5", features = ["util"] }
```

**Step 3: Commit**

```bash
git add echo-server/Cargo.toml
git commit -m "feat(echo-server): create Cargo.toml with dependencies"
```

---

### Task 2: Write the echo server main.rs

**Files:**
- Modify: `echo-server/src/main.rs`

**Step 1: Write the main.rs implementation**

```rust
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
```

**Step 2: Run build to verify compilation**

```bash
cd echo-server && cargo build
```

Expected: Compiles successfully

**Step 3: Commit**

```bash
git add echo-server/src/main.rs
git commit -m "feat(echo-server): implement echo handler that saves payloads"
```

---

### Task 3: Create BUILD.bazel file

**Files:**
- Create: `echo-server/BUILD.bazel`

**Step 1: Write BUILD.bazel**

```bazel
load("@rules_rust//rust:defs.bzl", "rust_binary")

rust_binary(
    name = "echo_server",
    srcs = ["src/main.rs"],
    cargo_toml = "//echo-server:Cargo.toml",
    deps = [
        "@crates//:axum",
        "@crates//:tokio",
        "@crates//:serde",
        "@crates//:serde_json",
        "@crates//:uuid",
        "@crates//:chrono",
        "@crates//:tower",
    ],
)
```

**Step 2: Verify bazel can resolve the dependencies**

```bash
bazel build //echo-server:echo_server
```

Expected: Builds successfully

**Step 3: Commit**

```bash
git add echo-server/BUILD.bazel
git commit -m "feat(echo-server): add BUILD.bazel for bazel build"
```

---

### Task 4: Test the server

**Step 1: Run the server**

```bash
bazel run //echo-server:echo_server
```

Expected: Server starts, prints "Echo server listening on http://0.0.0.0:8080"

**Step 2: Send a test request**

In another terminal:

```bash
curl -X POST http://localhost:8080/ \
  -H "Content-Type: application/json" \
  -d '{"test": "payload", "value": 123}'
```

Expected: Returns the same JSON body

**Step 3: Verify file was saved**

```bash
ls -la /tmp/gradle-payloads/
```

Expected: New JSON file with timestamp and payload content

**Step 4: Commit**

```bash
git commit --allow-empty -m "test(echo-server): verified server works correctly"
```

---

### Task 5: Final commit and summary

**Step 1: Verify all changes are committed**

```bash
git status
git log --oneline -5
```

Expected: All echo-server files committed

**Step 2: Done!**

The echo server is ready. Run it with:
```bash
bazel run //echo-server:echo_server
```

Then configure your Gradle build to point at `http://localhost:8080` to capture its payloads.
