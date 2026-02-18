# Design: Gradle Build Scan Echo Server

## Overview

A simple HTTP server that echoes requests back to clients while saving all incoming payloads to disk for later analysis. Used to reverse engineer the Gradle develocity client payloads.

## Architecture

**Single binary**: `//echo-server`  
**Language**: Rust with axum  
**Port**: 8080

## Components

1. **HTTP Server** - Axum-based async HTTP listener on port 8080
2. **Request Handler** - Captures request body, saves to disk, returns echo response
3. **Storage** - Files saved to `/tmp/gradle-payloads/<timestamp>-<uuid>.json`

## Data Flow

```
Client Request → Axum Server → Save body to disk → Echo response back
```

## Implementation Details

- **Capture**: Method, path, headers, body (full request)
- **Save format**: JSON with metadata + payload
- **Response**: Echo same body back with 200 OK
- **No storage needed**: Just saves files, no database

## File Structure

```
echo-server/
├── BUILD.bazel       # Bazel build rule
├── Cargo.toml        # Dependencies (axum)
└── src/
    └── main.rs       # Server implementation
```

## Dependencies

```toml
axum = "0.8"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
serde = "1"
serde_json = "1"
uuid = { version = "1", features = ["v4"] }
chrono = "0.4"
```