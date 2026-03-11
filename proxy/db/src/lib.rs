use std::str::FromStr;

use anyhow::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{FromRow, SqlitePool};

use domain::{Header, Payload, PayloadId, RequestData, ResponseData};
use migrations::MIGRATOR;

pub async fn connect(database_url: &str) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .pragma("foreign_keys", "ON");

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    MIGRATOR.run(&pool).await?;

    Ok(pool)
}

pub async fn insert_payload(pool: &SqlitePool, payload: &Payload) -> Result<()> {
    let id = payload.id.0.to_string();
    let request_headers = serde_json::to_string(
        &payload
            .request
            .headers
            .iter()
            .map(|h| serde_json::json!({"name": h.name, "value": h.value}))
            .collect::<Vec<_>>(),
    )?;
    let response_headers = payload
        .response
        .headers
        .as_ref()
        .map(|headers| {
            serde_json::to_string(
                &headers
                    .iter()
                    .map(|h| serde_json::json!({"name": h.name, "value": h.value}))
                    .collect::<Vec<_>>(),
            )
        })
        .transpose()?;

    sqlx::query(
        "INSERT INTO payloads (id, request_id, timestamp, method, uri, request_headers, request_body, response_status, response_headers, response_body, response_error)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&payload.request_id)
    .bind(&payload.timestamp)
    .bind(&payload.request.method)
    .bind(&payload.request.uri)
    .bind(&request_headers)
    .bind(&payload.request.body)
    .bind(payload.response.status)
    .bind(&response_headers)
    .bind(&payload.response.body)
    .bind(&payload.response.error)
    .execute(pool)
    .await?;

    Ok(())
}

#[derive(FromRow)]
struct PayloadRow {
    id: String,
    request_id: String,
    timestamp: String,
    method: String,
    uri: String,
    request_headers: String,
    request_body: Option<String>,
    response_status: Option<i32>,
    response_headers: Option<String>,
    response_body: Option<String>,
    response_error: Option<String>,
    created_at: String,
}

fn parse_headers(json: &str) -> Vec<Header> {
    serde_json::from_str::<Vec<serde_json::Value>>(json)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| {
            Some(Header {
                name: v.get("name")?.as_str()?.to_string(),
                value: v.get("value")?.as_str()?.to_string(),
            })
        })
        .collect()
}

fn row_to_payload(row: PayloadRow) -> Result<Payload> {
    let id = PayloadId(row.id.parse()?);
    Ok(Payload {
        id,
        request_id: row.request_id,
        timestamp: row.timestamp,
        request: RequestData {
            method: row.method,
            uri: row.uri,
            headers: parse_headers(&row.request_headers),
            body: row.request_body,
        },
        response: ResponseData {
            status: row.response_status,
            headers: row.response_headers.as_deref().map(parse_headers),
            body: row.response_body,
            error: row.response_error,
        },
        created_at: row.created_at,
    })
}

pub async fn get_payload(pool: &SqlitePool, id: &str) -> Result<Option<Payload>> {
    let row = sqlx::query_as::<_, PayloadRow>(
        "SELECT id, request_id, timestamp, method, uri, request_headers, request_body, response_status, response_headers, response_body, response_error, created_at FROM payloads WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    row.map(row_to_payload).transpose()
}

pub async fn list_payloads(
    pool: &SqlitePool,
    limit: i64,
    after_id: Option<&str>,
) -> Result<Vec<Payload>> {
    // Order by rowid (monotonically increasing insertion order) for chronological pagination.
    // UUID-based ordering would produce random page order since UUIDs are v4 (random).
    let rows = if let Some(after_id) = after_id {
        sqlx::query_as::<_, PayloadRow>(
            "SELECT id, request_id, timestamp, method, uri, request_headers, request_body, response_status, response_headers, response_body, response_error, created_at FROM payloads WHERE rowid > (SELECT rowid FROM payloads WHERE id = ?) ORDER BY rowid ASC LIMIT ?"
        )
        .bind(after_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, PayloadRow>(
            "SELECT id, request_id, timestamp, method, uri, request_headers, request_body, response_status, response_headers, response_body, response_error, created_at FROM payloads ORDER BY rowid ASC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(pool)
        .await?
    };

    rows.into_iter().map(row_to_payload).collect()
}

pub async fn count_payloads(pool: &SqlitePool) -> Result<i64> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM payloads")
        .fetch_one(pool)
        .await?;
    Ok(count)
}
