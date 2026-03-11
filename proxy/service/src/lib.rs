extern crate format as proxy_format;

use anyhow::Result;
use sqlx::SqlitePool;
use uuid::Uuid;

use domain::{Header, Payload, PayloadId, RequestData, ResponseData};

pub struct ProxyService {
    pool: SqlitePool,
}

impl ProxyService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn store_payload(&self, captured: &proxy_format::Payload) -> Result<()> {
        let payload = to_domain(captured);
        db::insert_payload(&self.pool, &payload).await
    }

    pub async fn get_payload(&self, id: &str) -> Result<Option<Payload>> {
        db::get_payload(&self.pool, id).await
    }

    pub async fn list_payloads(&self, limit: i64, after_id: Option<&str>) -> Result<Vec<Payload>> {
        db::list_payloads(&self.pool, limit, after_id).await
    }

    pub async fn count_payloads(&self) -> Result<i64> {
        db::count_payloads(&self.pool).await
    }
}

fn to_domain(captured: &proxy_format::Payload) -> Payload {
    let body = match &captured.request.body {
        serde_json::Value::Null => None,
        v => Some(v.to_string()),
    };
    let response_body = captured.response.body.as_ref().and_then(|v| {
        if v.is_null() {
            None
        } else {
            Some(v.to_string())
        }
    });

    Payload {
        id: PayloadId(Uuid::new_v4()),
        request_id: captured.request_id.clone(),
        timestamp: captured.timestamp.clone(),
        request: RequestData {
            method: captured.request.method.clone(),
            uri: captured.request.uri.clone(),
            headers: captured
                .request
                .headers
                .iter()
                .map(|(n, v)| Header {
                    name: n.clone(),
                    value: v.clone(),
                })
                .collect(),
            body,
        },
        response: ResponseData {
            status: captured.response.status.map(|s| s as i32),
            headers: captured.response.headers.as_ref().map(|hs| {
                hs.iter()
                    .map(|(n, v)| Header {
                        name: n.clone(),
                        value: v.clone(),
                    })
                    .collect()
            }),
            body: response_body,
            error: captured.response.error.clone(),
        },
        created_at: String::new(), // DB default fills this
    }
}
