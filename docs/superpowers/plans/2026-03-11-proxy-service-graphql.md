# Proxy Service with GraphQL API — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert the proxy from a JSON-file-dumping interceptor into a service with SQLite persistence and a Juniper GraphQL API.

**Architecture:** Layered crates mirroring the build-scan server: domain → db → service → graphql. Proxy remains a separate binary. Relay utilities duplicated from build-scan server (~200 lines). Activation gated on `UPSTREAM_URL` env var.

**Tech Stack:** Rust, Axum, sqlx (SQLite), Juniper, juniper_axum, Bazel (rules_rust + gazelle)

**Spec:** `docs/superpowers/specs/2026-03-11-proxy-service-graphql-design.md`

**Important Bazel/Rust notes:**
- This project has NO Cargo workspace. `Cargo.toml` is a flat dependency manifest for Bazel's `crates_repository`. Do NOT add workspace members.
- `sqlx::migrate!()` and `sqlx::query_as!()` compile-time macros do NOT work under Bazel's sandbox. Use runtime equivalents: `sqlx::query_as::<_, Row>()` with `#[derive(sqlx::FromRow)]`, and manually-constructed `Migrator` with `include_str!`.
- The `format` crate name conflicts with Rust's built-in `format!` macro. Use `extern crate format as proxy_format;` in crates that depend on it.

---

## File Structure

### New files

| File | Responsibility |
|------|---------------|
| `proxy/domain/src/lib.rs` | Domain model: `Payload`, `RequestData`, `ResponseData`, `Header` structs |
| `proxy/domain/src/BUILD.bazel` | Bazel library target |
| `proxy/db/migrations/src/sql/001_create_payloads.sql` | SQL migration |
| `proxy/db/migrations/src/lib.rs` | Bazel-compatible migrator with `include_str!` |
| `proxy/db/migrations/src/BUILD.bazel` | Bazel library target with `compile_data` |
| `proxy/db/src/lib.rs` | SQLite persistence: insert, list, get, count |
| `proxy/db/src/BUILD.bazel` | Bazel library target |
| `proxy/service/src/lib.rs` | Business logic: capture + store payloads, read queries |
| `proxy/service/src/BUILD.bazel` | Bazel library target |
| `proxy/graphql/relay/src/lib.rs` | Duplicated Relay utilities (RelayId, Cursor, pagination) |
| `proxy/graphql/relay/src/BUILD.bazel` | Bazel library + test targets |
| `proxy/graphql/src/lib.rs` | Juniper schema, resolvers, handler |
| `proxy/graphql/src/BUILD.bazel` | Bazel library target |

### Modified files

| File | Change |
|------|--------|
| `proxy/config/src/lib.rs` | Add `database_url`, remove `payload_dir` |
| `proxy/src/main.rs` | Replace JSON dumping with DB persistence, add GraphQL routes |
| `proxy/src/BUILD.bazel` | Add new crate dependencies |

---

## Chunk 1: Foundation (domain + db + migrations)

### Task 1: Domain models

**Files:**
- Create: `proxy/domain/src/lib.rs`
- Create: `proxy/domain/src/BUILD.bazel`

- [ ] **Step 1: Create domain model structs**

`proxy/domain/src/lib.rs`:
```rust
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PayloadId(pub Uuid);

#[derive(Debug, Clone)]
pub struct Header {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct RequestData {
    pub method: String,
    pub uri: String,
    pub headers: Vec<Header>,
    pub body: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResponseData {
    pub status: Option<i32>,
    pub headers: Option<Vec<Header>>,
    pub body: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Payload {
    pub id: PayloadId,
    pub request_id: String,
    pub timestamp: String,
    pub request: RequestData,
    pub response: ResponseData,
    pub created_at: String,
}
```

- [ ] **Step 2: Create BUILD.bazel**

`proxy/domain/src/BUILD.bazel`:
```starlark
load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
    name = "domain",
    srcs = ["lib.rs"],
    visibility = ["//visibility:public"],
    deps = [
        "@crates//:uuid",
    ],
)
```

- [ ] **Step 3: Build to verify**

Run: `aspect build //proxy/domain/src:domain`
Expected: BUILD SUCCESS

- [ ] **Step 4: Run gazelle and format**

Run: `bazel run gazelle && bazel run //tools/format`

- [ ] **Step 5: Commit**

```bash
git add proxy/domain/
git commit -m "feat(proxy): add domain model crate"
```

---

### Task 2: Database migration

**Files:**
- Create: `proxy/db/migrations/src/sql/001_create_payloads.sql`
- Create: `proxy/db/migrations/src/lib.rs`
- Create: `proxy/db/migrations/src/BUILD.bazel`

- [ ] **Step 1: Create SQL migration**

`proxy/db/migrations/src/sql/001_create_payloads.sql`:
```sql
CREATE TABLE IF NOT EXISTS payloads (
    id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    method TEXT NOT NULL,
    uri TEXT NOT NULL,
    request_headers TEXT NOT NULL,
    request_body TEXT,
    response_status INTEGER,
    response_headers TEXT,
    response_body TEXT,
    response_error TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

- [ ] **Step 2: Create Bazel-compatible migrator**

`proxy/db/migrations/src/lib.rs`:
```rust
use std::borrow::Cow;

use sqlx::migrate::{Migration, MigrationType, Migrator};

/// Bazel-compatible migrator: sqlx::migrate!() relies on CARGO_MANIFEST_DIR
/// which doesn't resolve correctly in Bazel's sandbox. We construct the Migrator
/// manually using include_str! (which Bazel handles via compile_data).
pub static MIGRATOR: Migrator = Migrator {
    migrations: Cow::Borrowed(&[Migration {
        version: 1,
        description: Cow::Borrowed("create payloads"),
        migration_type: MigrationType::Simple,
        sql: Cow::Borrowed(include_str!("sql/001_create_payloads.sql")),
        checksum: Cow::Borrowed(&[]),
        no_tx: false,
    }]),
    ignore_missing: false,
    locking: true,
    no_tx: false,
};
```

- [ ] **Step 3: Create BUILD.bazel**

`proxy/db/migrations/src/BUILD.bazel`:
```starlark
load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
    name = "migrations",
    srcs = ["lib.rs"],
    compile_data = [
        "sql/001_create_payloads.sql",
    ],
    visibility = ["//visibility:public"],
    deps = ["@crates//:sqlx"],
)
```

- [ ] **Step 4: Build to verify**

Run: `aspect build //proxy/db/migrations/src:migrations`
Expected: BUILD SUCCESS

- [ ] **Step 5: Run gazelle and format**

Run: `bazel run gazelle && bazel run //tools/format`

- [ ] **Step 6: Commit**

```bash
git add proxy/db/migrations/
git commit -m "feat(proxy): add database migration for payloads table"
```

---

### Task 3: Database layer

**Files:**
- Create: `proxy/db/src/lib.rs`
- Create: `proxy/db/src/BUILD.bazel`

- [ ] **Step 1: Create db crate with connect + insert + query functions**

`proxy/db/src/lib.rs`:
```rust
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
    let response_headers = payload.response.headers.as_ref().map(|headers| {
        serde_json::to_string(
            &headers
                .iter()
                .map(|h| serde_json::json!({"name": h.name, "value": h.value}))
                .collect::<Vec<_>>(),
        )
        .unwrap_or_default()
    });

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
    let rows = if let Some(after_id) = after_id {
        sqlx::query_as::<_, PayloadRow>(
            "SELECT id, request_id, timestamp, method, uri, request_headers, request_body, response_status, response_headers, response_body, response_error, created_at FROM payloads WHERE id > ? ORDER BY id ASC LIMIT ?"
        )
        .bind(after_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, PayloadRow>(
            "SELECT id, request_id, timestamp, method, uri, request_headers, request_body, response_status, response_headers, response_body, response_error, created_at FROM payloads ORDER BY id ASC LIMIT ?"
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
```

- [ ] **Step 2: Create BUILD.bazel**

`proxy/db/src/BUILD.bazel`:
```starlark
load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
    name = "db",
    srcs = ["lib.rs"],
    visibility = ["//visibility:public"],
    deps = [
        "//proxy/db/migrations/src:migrations",
        "//proxy/domain/src:domain",
        "@crates//:anyhow",
        "@crates//:serde_json",
        "@crates//:sqlx",
        "@crates//:uuid",
    ],
)
```

- [ ] **Step 3: Build to verify**

Run: `aspect build //proxy/db/src:db`
Expected: BUILD SUCCESS

- [ ] **Step 4: Run gazelle and format**

Run: `bazel run gazelle && bazel run //tools/format`

- [ ] **Step 5: Commit**

```bash
git add proxy/db/
git commit -m "feat(proxy): add database layer with payload CRUD"
```

---

## Chunk 2: Service + Relay + GraphQL

### Task 4: Service layer

**Files:**
- Create: `proxy/service/src/lib.rs`
- Create: `proxy/service/src/BUILD.bazel`

- [ ] **Step 1: Create service crate**

Note: The `format` crate name conflicts with Rust's built-in `format!` macro. Use `extern crate` to alias it.

`proxy/service/src/lib.rs`:
```rust
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

    pub async fn list_payloads(
        &self,
        limit: i64,
        after_id: Option<&str>,
    ) -> Result<Vec<Payload>> {
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
    let response_body = captured
        .response
        .body
        .as_ref()
        .and_then(|v| if v.is_null() { None } else { Some(v.to_string()) });

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
```

- [ ] **Step 2: Create BUILD.bazel**

`proxy/service/src/BUILD.bazel`:
```starlark
load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
    name = "service",
    srcs = ["lib.rs"],
    visibility = ["//visibility:public"],
    deps = [
        "//proxy/db/src:db",
        "//proxy/domain/src:domain",
        "//proxy/format/src:format",
        "@crates//:anyhow",
        "@crates//:serde_json",
        "@crates//:sqlx",
        "@crates//:uuid",
    ],
)
```

- [ ] **Step 3: Build to verify**

Run: `aspect build //proxy/service/src:service`
Expected: BUILD SUCCESS

- [ ] **Step 4: Run gazelle and format**

Run: `bazel run gazelle && bazel run //tools/format`

- [ ] **Step 5: Commit**

```bash
git add proxy/service/
git commit -m "feat(proxy): add service layer for payload storage and queries"
```

---

### Task 5: Relay utilities

**Files:**
- Create: `proxy/graphql/relay/src/lib.rs`
- Create: `proxy/graphql/relay/src/BUILD.bazel`

- [ ] **Step 1: Duplicate relay utilities from build-scan server**

Copy `build-scan/server/graphql/relay/src/lib.rs` to `proxy/graphql/relay/src/lib.rs` verbatim. This is ~208 lines containing `RelayId`, `Cursor`, `validate_pagination`, and tests. No modifications needed.

- [ ] **Step 2: Create BUILD.bazel**

`proxy/graphql/relay/src/BUILD.bazel`:
```starlark
load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

rust_library(
    name = "relay",
    srcs = ["lib.rs"],
    visibility = ["//visibility:public"],
    deps = [
        "@crates//:anyhow",
        "@crates//:base64",
        "@crates//:juniper",
        "@crates//:serde",
        "@crates//:serde_json",
    ],
)

rust_test(
    name = "relay_test",
    crate = ":relay",
)
```

- [ ] **Step 3: Build and test**

Run: `aspect build //proxy/graphql/relay/src:relay && aspect test //proxy/graphql/relay/src:relay_test`
Expected: BUILD SUCCESS, all tests pass

- [ ] **Step 4: Run gazelle and format**

Run: `bazel run gazelle && bazel run //tools/format`

- [ ] **Step 5: Commit**

```bash
git add proxy/graphql/relay/
git commit -m "feat(proxy): add Relay utilities (duplicated from build-scan server)"
```

---

### Task 6: GraphQL schema and resolvers

**Files:**
- Create: `proxy/graphql/src/lib.rs`
- Create: `proxy/graphql/src/BUILD.bazel`

- [ ] **Step 1: Create GraphQL crate**

This follows the exact patterns from `build-scan/server/graphql/src/lib.rs`. Key patterns:
- All types use `graphql_object` proc macro (NOT `#[derive(GraphQLObject)]`)
- Error conversion: `.map_err(|e| FieldError::from(e.to_string()))?` on every service/relay call
- Edge `node()` returns `&Type` (reference)
- Connection `edges()` returns `&Vec<Edge>` (reference)
- `RelayId::decode(&id)` returns `Result<RelayId>` with `.type_name` and `.raw_id` fields
- `RelayId::encode("Type", &id_str)` returns `juniper::ID` directly
- Schema type has no lifetime parameter

`proxy/graphql/src/lib.rs`:
```rust
use std::sync::Arc;

use axum::Extension;
use juniper::{
    EmptyMutation, EmptySubscription, FieldError, FieldResult, RootNode, ID, graphql_object,
};
use juniper_axum::{extract::JuniperRequest, response::JuniperResponse};

use domain;
use relay::{Cursor, RelayId, validate_pagination};
use service::ProxyService;

pub struct Context {
    pub service: Arc<ProxyService>,
}

impl juniper::Context for Context {}

// ---------------------------------------------------------------------------
// Payload type
// ---------------------------------------------------------------------------

pub struct Payload {
    pub payload: domain::Payload,
}

#[graphql_object(context = Context)]
impl Payload {
    fn id(&self) -> ID {
        RelayId::encode("Payload", &self.payload.id.0.to_string())
    }

    fn request_id(&self) -> &str {
        &self.payload.request_id
    }

    fn timestamp(&self) -> &str {
        &self.payload.timestamp
    }

    fn request(&self) -> RequestData {
        RequestData {
            method: self.payload.request.method.clone(),
            uri: self.payload.request.uri.clone(),
            headers: self
                .payload
                .request
                .headers
                .iter()
                .map(|h| Header {
                    name: h.name.clone(),
                    value: h.value.clone(),
                })
                .collect(),
            body: self.payload.request.body.clone(),
        }
    }

    fn response(&self) -> ResponseData {
        ResponseData {
            status: self.payload.response.status,
            headers: self.payload.response.headers.as_ref().map(|hs| {
                hs.iter()
                    .map(|h| Header {
                        name: h.name.clone(),
                        value: h.value.clone(),
                    })
                    .collect()
            }),
            body: self.payload.response.body.clone(),
            error: self.payload.response.error.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Nested value types
// ---------------------------------------------------------------------------

pub struct Header {
    pub name: String,
    pub value: String,
}

#[graphql_object(context = Context)]
impl Header {
    fn name(&self) -> &str {
        &self.name
    }

    fn value(&self) -> &str {
        &self.value
    }
}

pub struct RequestData {
    pub method: String,
    pub uri: String,
    pub headers: Vec<Header>,
    pub body: Option<String>,
}

#[graphql_object(context = Context)]
impl RequestData {
    fn method(&self) -> &str {
        &self.method
    }

    fn uri(&self) -> &str {
        &self.uri
    }

    fn headers(&self) -> &Vec<Header> {
        &self.headers
    }

    fn body(&self) -> Option<&str> {
        self.body.as_deref()
    }
}

pub struct ResponseData {
    pub status: Option<i32>,
    pub headers: Option<Vec<Header>>,
    pub body: Option<String>,
    pub error: Option<String>,
}

#[graphql_object(context = Context)]
impl ResponseData {
    fn status(&self) -> Option<i32> {
        self.status
    }

    fn headers(&self) -> &Option<Vec<Header>> {
        &self.headers
    }

    fn body(&self) -> Option<&str> {
        self.body.as_deref()
    }

    fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

// ---------------------------------------------------------------------------
// PageInfo
// ---------------------------------------------------------------------------

pub struct PageInfo {
    pub has_next_page: bool,
    pub end_cursor: Option<String>,
}

#[graphql_object(context = Context)]
impl PageInfo {
    fn has_next_page(&self) -> bool {
        self.has_next_page
    }

    fn end_cursor(&self) -> Option<&str> {
        self.end_cursor.as_deref()
    }
}

// ---------------------------------------------------------------------------
// PayloadConnection / PayloadEdge
// ---------------------------------------------------------------------------

pub struct PayloadEdge {
    pub cursor: String,
    pub node: Payload,
}

#[graphql_object(context = Context)]
impl PayloadEdge {
    fn cursor(&self) -> &str {
        &self.cursor
    }

    fn node(&self) -> &Payload {
        &self.node
    }
}

pub struct PayloadConnection {
    pub edges: Vec<PayloadEdge>,
    pub page_info: PageInfo,
}

#[graphql_object(context = Context)]
impl PayloadConnection {
    fn edges(&self) -> &Vec<PayloadEdge> {
        &self.edges
    }

    fn page_info(&self) -> &PageInfo {
        &self.page_info
    }

    async fn total_count(&self, context: &Context) -> FieldResult<i32> {
        let count = context
            .service
            .count_payloads()
            .await
            .map_err(|e| FieldError::from(e.to_string()))? as i32;
        Ok(count)
    }
}

// ---------------------------------------------------------------------------
// QueryRoot
// ---------------------------------------------------------------------------

pub struct QueryRoot;

#[graphql_object(context = Context)]
impl QueryRoot {
    async fn node(context: &Context, id: ID) -> FieldResult<Option<Payload>> {
        let relay_id =
            RelayId::decode(&id).map_err(|e| FieldError::from(e.to_string()))?;
        match relay_id.type_name.as_str() {
            "Payload" => {
                let payload = context
                    .service
                    .get_payload(&relay_id.raw_id)
                    .await
                    .map_err(|e| FieldError::from(e.to_string()))?;
                Ok(payload.map(|p| Payload { payload: p }))
            }
            _ => Ok(None),
        }
    }

    async fn payload(context: &Context, id: ID) -> FieldResult<Option<Payload>> {
        // Support both raw UUID and Relay global ID
        let raw_id = if let Ok(relay_id) = RelayId::decode(&id) {
            relay_id.raw_id
        } else {
            id.to_string()
        };

        let payload = context
            .service
            .get_payload(&raw_id)
            .await
            .map_err(|e| FieldError::from(e.to_string()))?;

        Ok(payload.map(|p| Payload { payload: p }))
    }

    async fn payloads(
        context: &Context,
        first: Option<i32>,
        after: Option<String>,
    ) -> FieldResult<PayloadConnection> {
        let limit =
            validate_pagination(first).map_err(|e| FieldError::from(e.to_string()))?;
        let fetch_limit = (limit + 1) as i64;

        let after_id = after
            .as_deref()
            .map(Cursor::decode)
            .transpose()
            .map_err(|e| FieldError::from(e.to_string()))?
            .map(|c| c.value);

        let mut payloads = context
            .service
            .list_payloads(fetch_limit, after_id.as_deref())
            .await
            .map_err(|e| FieldError::from(e.to_string()))?;

        let has_next_page = payloads.len() > limit as usize;
        if has_next_page {
            payloads.pop();
        }

        let end_cursor = payloads
            .last()
            .map(|p| Cursor::new(p.id.0.to_string()).encode());

        let edges: Vec<PayloadEdge> = payloads
            .into_iter()
            .map(|p| {
                let cursor = Cursor::new(p.id.0.to_string()).encode();
                PayloadEdge {
                    cursor,
                    node: Payload { payload: p },
                }
            })
            .collect();

        Ok(PayloadConnection {
            edges,
            page_info: PageInfo {
                has_next_page,
                end_cursor,
            },
        })
    }
}

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

pub type Schema = RootNode<QueryRoot, EmptyMutation<Context>, EmptySubscription<Context>>;

pub fn create_schema() -> Schema {
    Schema::new(QueryRoot, EmptyMutation::new(), EmptySubscription::new())
}

pub async fn graphql_handler(
    Extension(schema): Extension<Arc<Schema>>,
    Extension(context): Extension<Arc<Context>>,
    JuniperRequest(request): JuniperRequest,
) -> JuniperResponse {
    JuniperResponse(request.execute(&*schema, &*context).await)
}
```

- [ ] **Step 2: Create BUILD.bazel**

`proxy/graphql/src/BUILD.bazel`:
```starlark
load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
    name = "graphql",
    srcs = ["lib.rs"],
    visibility = ["//visibility:public"],
    deps = [
        "//proxy/domain/src:domain",
        "//proxy/graphql/relay/src:relay",
        "//proxy/service/src:service",
        "@crates//:axum",
        "@crates//:juniper",
        "@crates//:juniper_axum",
    ],
)
```

- [ ] **Step 3: Build to verify**

Run: `aspect build //proxy/graphql/src:graphql`
Expected: BUILD SUCCESS

- [ ] **Step 4: Run gazelle and format**

Run: `bazel run gazelle && bazel run //tools/format`

- [ ] **Step 5: Commit**

```bash
git add proxy/graphql/
git commit -m "feat(proxy): add GraphQL schema with Relay pagination"
```

---

## Chunk 3: Wire it up (config + main)

### Task 7: Update config

**Files:**
- Modify: `proxy/config/src/lib.rs`

- [ ] **Step 1: Update config to add database_url, remove payload_dir**

Replace the contents of `proxy/config/src/lib.rs`:
```rust
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub upstream_url: String,
    pub database_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        let port = env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);

        let upstream_url =
            env::var("UPSTREAM_URL").expect("UPSTREAM_URL environment variable is required");
        let upstream_url = upstream_url.trim_end_matches('/').to_string();

        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:proxy.db".to_string());

        Config {
            port,
            upstream_url,
            database_url,
        }
    }
}
```

- [ ] **Step 2: Build to verify**

Run: `aspect build //proxy/config/src:config`
Expected: BUILD SUCCESS

- [ ] **Step 3: Run gazelle and format**

Run: `bazel run gazelle && bazel run //tools/format`

- [ ] **Step 4: Commit**

```bash
git add proxy/config/
git commit -m "feat(proxy): update config with database_url, remove payload_dir"
```

---

### Task 8: Update main.rs

**Files:**
- Modify: `proxy/src/main.rs`
- Modify: `proxy/src/BUILD.bazel`

- [ ] **Step 1: Rewrite main.rs**

The new main.rs replaces JSON file dumping with `service.store_payload()` and adds GraphQL routes. The proxy handler logic (forwarding, header filtering, body capture) stays the same. `base64` is still needed for encoding binary request/response bodies.

Key architectural note: The proxy handler uses `State<AppState>` (Axum state) while GraphQL uses `Extension` (Axum layer). Both patterns coexist — `AppState` holds the `reqwest::Client`, config, and `Arc<ProxyService>`, while `Extension` holds the GraphQL schema and context.

Replace contents of `proxy/src/main.rs`:
```rust
extern crate format as proxy_format;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{Request, State};
use axum::response::Response;
use axum::body::Body;
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
            axum::routing::get(graphql::graphql_handler)
                .post(graphql::graphql_handler),
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
        info!("Stored payload {} for {} {}", request_id, method, path_and_query);
    }

    http_response
}
```

- [ ] **Step 2: Update BUILD.bazel**

Replace contents of `proxy/src/BUILD.bazel`:
```starlark
load("@rules_rust//rust:defs.bzl", "rust_binary")

rust_binary(
    name = "main",
    srcs = ["main.rs"],
    deps = [
        "//proxy/config/src:config",
        "//proxy/db/src:db",
        "//proxy/format/src:format",
        "//proxy/graphql/src:graphql",
        "//proxy/service/src:service",
        "@crates//:axum",
        "@crates//:base64",
        "@crates//:chrono",
        "@crates//:juniper_axum",
        "@crates//:reqwest",
        "@crates//:serde_json",
        "@crates//:tokio",
        "@crates//:tracing",
        "@crates//:tracing-subscriber",
        "@crates//:uuid",
    ],
)
```

- [ ] **Step 3: Build to verify**

Run: `aspect build //proxy/src:main`
Expected: BUILD SUCCESS

- [ ] **Step 4: Run gazelle and format**

Run: `bazel run gazelle && bazel run //tools/format`

- [ ] **Step 5: Smoke test**

Run: `UPSTREAM_URL=http://httpbin.org bazel run //proxy/src:main`
Expected: Server starts on port 8080, logs show DB connection. Open `http://localhost:8080/graphiql` — GraphiQL IDE should load.

- [ ] **Step 6: Commit**

```bash
git add proxy/src/ proxy/config/
git commit -m "feat(proxy): wire up SQLite persistence and GraphQL endpoint"
```

---

### Task 9: Full build, test, and PR

- [ ] **Step 1: Run full build**

Run: `aspect build //...`
Expected: BUILD SUCCESS

- [ ] **Step 2: Run full tests**

Run: `aspect test //...`
Expected: All tests pass

- [ ] **Step 3: Run format and lint**

Run: `bazel run //tools/format && aspect lint --fix`

- [ ] **Step 4: Commit formatting changes if any**

```bash
git add proxy/
git commit -m "chore: format and lint"
```

- [ ] **Step 5: Create PR**

```bash
gh pr create --title "feat(proxy): add SQLite persistence and GraphQL API" --body "$(cat <<'EOF'
## Summary
- Convert proxy from JSON file dumping to SQLite persistence
- Add GraphQL API with Relay pagination for querying captured payloads
- Follow same layered architecture as build-scan server (domain → db → service → graphql)
- Proxy remains a separate binary, activated by UPSTREAM_URL

## New crates
- `proxy/domain` — domain models
- `proxy/db` — SQLite persistence via sqlx
- `proxy/service` — business logic
- `proxy/graphql` — Juniper schema + resolvers
- `proxy/graphql/relay` — Relay utilities (duplicated from build-scan server)

## Test plan
- [ ] `aspect build //...` passes
- [ ] `aspect test //...` passes
- [ ] `UPSTREAM_URL=http://httpbin.org bazel run //proxy/src:main` starts server
- [ ] GraphiQL loads at http://localhost:8080/graphiql
- [ ] `{ payloads { edges { node { requestId request { method uri } } } totalCount } }` returns results after proxying requests
EOF
)"
```
