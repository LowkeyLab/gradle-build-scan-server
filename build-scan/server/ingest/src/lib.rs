use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use service::{BuildScanService, UploadRequest};
use uuid::Uuid;

pub mod mime {
    pub const SCAN_TOKEN_REQUEST: &str = "application/vnd.gradle.scan-token-request+json";
    pub const SCAN_ACK: &str = "application/vnd.gradle.scan-ack+json";
    pub const SCAN: &str = "application/vnd.gradle.scan";
    pub const SCAN_UPLOAD_ACK: &str = "application/vnd.gradle.scan-upload-ack+json";
    pub const SCAN_UPLOAD_FAILURE: &str = "application/vnd.gradle.scan-upload-failure+json";
    pub const USER_COUNT_CHECK_RESPONSE: &str =
        "application/vnd.gradle.user-count-check-response+json";
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenRequest {
    pub build_tool_type: Option<String>,
    pub build_tool_version: Option<String>,
    pub build_agent_version: Option<String>,
    pub payload_size: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenResponse {
    pub id: String,
    pub scan_upload_token: String,
    pub scan_upload_url: String,
    pub scan_url: String,
}

#[derive(Debug, Serialize)]
pub struct UploadAck {}

#[derive(Debug, Serialize)]
pub struct UploadFailure {
    pub message: String,
    pub retry: bool,
}

#[derive(Debug, Clone)]
pub struct PendingUpload {
    pub scan_id: String,
    pub upload_token: String,
    pub build_tool_type: Option<String>,
    pub build_tool_version: Option<String>,
    pub plugin_version: Option<String>,
    pub payload_size: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct IngestState {
    pending: Arc<Mutex<HashMap<String, PendingUpload>>>,
}

impl IngestState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_pending(&self, upload: PendingUpload) {
        self.pending
            .lock()
            .expect("pending uploads mutex should not be poisoned")
            .insert(upload.upload_token.clone(), upload);
    }

    pub fn take_pending(&self, upload_token: &str) -> Option<PendingUpload> {
        self.pending
            .lock()
            .expect("pending uploads mutex should not be poisoned")
            .remove(upload_token)
    }

    pub fn put_pending(&self, upload: PendingUpload) {
        self.insert_pending(upload);
    }
}

#[derive(Clone)]
pub struct IngestAppState {
    pub base_url: String,
    pub ingest: IngestState,
    pub service: Arc<BuildScanService>,
}

pub async fn handle_token_request(
    State(state): State<IngestAppState>,
    Path((tool_type, version)): Path<(String, String)>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !content_type.starts_with(mime::SCAN_TOKEN_REQUEST) {
        return (
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            format!(
                "Expected Content-Type starting with {}",
                mime::SCAN_TOKEN_REQUEST
            ),
        )
            .into_response();
    }

    let token_request: TokenRequest = match serde_json::from_slice(&body) {
        Ok(req) => req,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("Invalid request body: {e}"),
            )
                .into_response();
        }
    };

    let scan_id = Uuid::new_v4().to_string();
    let upload_token = Uuid::new_v4().to_string();

    let pending = PendingUpload {
        scan_id: scan_id.clone(),
        upload_token: upload_token.clone(),
        build_tool_type: token_request.build_tool_type.or(Some(tool_type.clone())),
        build_tool_version: token_request.build_tool_version,
        plugin_version: token_request.build_agent_version.or(Some(version.clone())),
        payload_size: token_request.payload_size,
    };
    state.ingest.insert_pending(pending);

    let response = TokenResponse {
        scan_upload_token: upload_token,
        scan_upload_url: format!(
            "{}/scans/publish/{}/{}/upload",
            state.base_url, tool_type, version
        ),
        scan_url: format!("{}/web/scans/{}", state.base_url, scan_id),
        id: scan_id,
    };

    let body = match serde_json::to_vec(&response) {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to serialize response: {e}"),
            )
                .into_response();
        }
    };

    axum::http::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", mime::SCAN_ACK)
        .body(axum::body::Body::from(body))
        .expect("response builder should not fail")
}

pub async fn handle_scan_upload(
    State(state): State<IngestAppState>,
    Path((_tool_type, _version)): Path<(String, String)>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let raw_token = match headers.get("x-upload-token").and_then(|v| v.to_str().ok()) {
        Some(t) => t.to_string(),
        None => {
            let failure = UploadFailure {
                message: "Missing X-Upload-Token header".to_string(),
                retry: false,
            };
            let body = serde_json::to_vec(&failure).unwrap_or_default();
            return axum::http::Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header("Content-Type", mime::SCAN_UPLOAD_FAILURE)
                .body(axum::body::Body::from(body))
                .expect("response builder should not fail");
        }
    };

    // The Gradle client base64-encodes the upload token (no-pad) before sending.
    // Try to decode it; fall back to the raw value if decoding fails.
    let upload_token = base64::engine::general_purpose::STANDARD_NO_PAD
        .decode(&raw_token)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .unwrap_or(raw_token);

    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if content_type != mime::SCAN {
        let failure = UploadFailure {
            message: format!("Expected Content-Type {}", mime::SCAN),
            retry: false,
        };
        let body = serde_json::to_vec(&failure).unwrap_or_default();
        return axum::http::Response::builder()
            .status(StatusCode::UNSUPPORTED_MEDIA_TYPE)
            .header("Content-Type", mime::SCAN_UPLOAD_FAILURE)
            .body(axum::body::Body::from(body))
            .expect("response builder should not fail");
    }

    let pending = match state.ingest.take_pending(&upload_token) {
        Some(p) => p,
        None => {
            let failure = UploadFailure {
                message: format!("No pending upload found for token: {upload_token}"),
                retry: false,
            };
            let body = serde_json::to_vec(&failure).unwrap_or_default();
            return axum::http::Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", mime::SCAN_UPLOAD_FAILURE)
                .body(axum::body::Body::from(body))
                .expect("response builder should not fail");
        }
    };

    if let Some(expected_size) = pending.payload_size {
        let actual_size = body.len() as u64;
        if actual_size > expected_size {
            state.ingest.put_pending(pending);
            let failure = UploadFailure {
                message: format!(
                    "Payload size {actual_size} exceeds declared size {expected_size}"
                ),
                retry: true,
            };
            let body = serde_json::to_vec(&failure).unwrap_or_default();
            return axum::http::Response::builder()
                .status(StatusCode::PAYLOAD_TOO_LARGE)
                .header("Content-Type", mime::SCAN_UPLOAD_FAILURE)
                .body(axum::body::Body::from(body))
                .expect("response builder should not fail");
        }
    }

    let req = UploadRequest {
        scan_id: pending.scan_id,
        build_tool_type: pending.build_tool_type,
        build_tool_version: pending.build_tool_version,
        plugin_version: pending.plugin_version,
        raw_payload: body.to_vec(),
    };

    let svc = state.service.clone();
    tokio::spawn(async move {
        if let Err(e) = svc.process_upload(req).await {
            tracing::error!(error = %e, "Failed to process build scan upload");
        }
    });

    let ack = UploadAck {};
    let ack_body = serde_json::to_vec(&ack).unwrap_or_else(|_| b"{}".to_vec());

    axum::http::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", mime::SCAN_UPLOAD_ACK)
        .body(axum::body::Body::from(ack_body))
        .expect("response builder should not fail")
}

pub async fn handle_usage_check() -> Response {
    let body = serde_json::json!({"numberOfDays": 0});
    let body_bytes = serde_json::to_vec(&body).unwrap_or_default();

    axum::http::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", mime::USER_COUNT_CHECK_RESPONSE)
        .body(axum::body::Body::from(body_bytes))
        .expect("response builder should not fail")
}
