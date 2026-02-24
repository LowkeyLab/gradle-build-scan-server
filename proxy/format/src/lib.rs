use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestData {
    pub method: String,
    pub uri: String,
    pub headers: Vec<(String, String)>,
    pub body: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<(String, String)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Payload {
    pub request_id: String,
    pub timestamp: String,
    pub request: RequestData,
    pub response: ResponseData,
}
