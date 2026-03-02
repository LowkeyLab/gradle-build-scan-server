# Domain Layer Refactoring Design

## Goal

Introduce a domain layer between the DB and GraphQL/service layers. Replace direct use of DB row structs with rich domain types using UUIDs, newtypes, strict enums, and chrono datetimes.

## Architecture

```
graphql  →  service  →  db
   ↓           ↓         ↓
         domain (pure types)
   ↑           ↑
 ingest  →  service
```

No layer depends upward. `domain` depends on nothing except `uuid` and `chrono`.

## New Crate: `server/domain/`

Pure Rust types. No framework dependencies (no sqlx, no juniper).

### Newtypes

```rust
// IDs
pub struct BuildScanId(pub Uuid);
pub struct TaskId(pub Uuid);

// String-wrapped value types
pub struct BuildToolType(pub String);
pub struct BuildToolVersion(pub String);
pub struct PluginVersion(pub String);
pub struct Hostname(pub String);
pub struct OsName(pub String);
pub struct OsVersion(pub String);
pub struct JvmVendor(pub String);
pub struct JvmVersion(pub String);
pub struct TaskPath(pub String);
pub struct ClassName(pub String);
pub struct CacheKey(pub String);
pub struct RequestedTask(pub String);

// Numeric value types
pub struct Timestamp(pub i64);   // epoch milliseconds, point in time
pub struct Duration(pub i64);    // milliseconds, elapsed time
```

### Enums (strict, no Unknown fallback)

```rust
pub enum BuildOutcome {
    Success,
    Failed,
    ParseError,
}

pub enum TaskOutcome {
    Success,
    Failed,
    UpToDate,
    Skipped,
    FromCache,
    NoSource,
    AvoidedForUnknownReason,
}
```

Mapping from unrecognized strings returns `Err`, not a fallback variant.

### Aggregates

```rust
pub struct BuildScan {
    pub id: BuildScanId,
    pub build_tool_type: BuildToolType,
    pub build_tool_version: BuildToolVersion,
    pub plugin_version: PluginVersion,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub outcome: Option<BuildOutcome>,
    pub requested_tasks: Option<Vec<RequestedTask>>,
    pub hostname: Option<Hostname>,
    pub os_name: Option<OsName>,
    pub os_version: Option<OsVersion>,
    pub jvm_vendor: Option<JvmVendor>,
    pub jvm_version: Option<JvmVersion>,
    pub created_at: DateTime<Utc>,
}

pub struct Task {
    pub id: TaskId,
    pub scan_id: BuildScanId,
    pub task_path: TaskPath,
    pub class_name: Option<ClassName>,
    pub outcome: Option<TaskOutcome>,
    pub cacheable: Option<bool>,
    pub start_timestamp: Option<Timestamp>,
    pub finish_timestamp: Option<Timestamp>,
    pub cache_key: Option<CacheKey>,
    pub origin_execution_time: Option<Duration>,
}
```

## DB Layer Changes (`server/db/`)

### DAOs with `sqlx::FromRow`

```rust
#[derive(sqlx::FromRow)]
pub struct BuildScanRow {
    pub id: String,
    pub build_tool_type: String,
    pub build_tool_version: String,
    pub plugin_version: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub outcome: Option<String>,
    pub requested_tasks: Option<String>,  // JSON
    pub hostname: Option<String>,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub jvm_vendor: Option<String>,
    pub jvm_version: Option<String>,
    pub created_at: String,
}

#[derive(sqlx::FromRow)]
pub struct TaskRow {
    pub id: String,
    pub scan_id: String,
    pub task_path: String,
    pub class_name: Option<String>,
    pub outcome: Option<String>,
    pub cacheable: Option<bool>,
    pub start_timestamp: Option<i64>,
    pub finish_timestamp: Option<i64>,
    pub cache_key: Option<String>,
    pub origin_execution_time: Option<i64>,
}
```

### Conversions

- `TryFrom<BuildScanRow> for domain::BuildScan` — fallible (UUID parse, datetime parse, strict enum parse, JSON deserialize)
- `TryFrom<TaskRow> for domain::Task` — fallible (UUID parse, strict enum parse)
- `From<&domain::BuildScan> for BuildScanRow` — infallible (serialize to strings)
- `From<&domain::Task> for TaskRow` — infallible

DB functions continue to use `sqlx::query_as::<_, BuildScanRow>(...)` internally.

## Service Layer Changes (`server/service/`)

Expanded to handle all reads. Returns domain types.

```rust
pub struct BuildScanService {
    pool: SqlitePool,
}

impl BuildScanService {
    pub fn new(pool: SqlitePool) -> Self;

    // Write
    pub async fn process_upload(&self, req: UploadRequest) -> Result<()>;

    // Reads (new)
    pub async fn get_build_scan(&self, id: &str) -> Result<Option<BuildScan>>;
    pub async fn list_build_scans(&self, limit: i64, after: Option<(String, String)>) -> Result<Vec<BuildScan>>;
    pub async fn get_task(&self, id: &str) -> Result<Option<Task>>;
    pub async fn list_tasks(&self, scan_id: &str, limit: i64, after: Option<String>) -> Result<Vec<Task>>;
    pub async fn count_tasks(&self, scan_id: &str) -> Result<i64>;
    pub async fn count_build_scans(&self) -> Result<i64>;
}
```

Each read method: calls DB function → gets Row → converts to domain type via `TryFrom`.

## GraphQL Layer Changes (`server/graphql/`)

### Context

```rust
pub struct Context {
    pub service: Arc<BuildScanService>,  // was: Arc<SqlitePool>
}
```

### Types wrap domain, not DB rows

```rust
pub struct BuildScan {
    pub scan: domain::BuildScan,  // was: pub row: db::BuildScanRow
}

pub struct Task {
    pub task: domain::Task,  // was: pub row: db::TaskRow
}
```

Resolvers call `context.service.*` instead of `db::*` directly.

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| sqlx derive approach | `#[derive(FromRow)]` | Simpler than compile-time checked queries; avoids Bazel integration issues |
| Domain crate location | Separate `server/domain/` crate | Clear dependency direction; prevents accidental coupling |
| Service scope | All reads + writes | Single gateway to data; GraphQL never touches DB directly |
| Service interface | Concrete struct | YAGNI; extract trait when mocking is needed |
| Datetime type | `DateTime<Utc>` | Semantically correct; these are absolute UTC timestamps |
| Outcome enums | Strict (no Unknown) | Fail fast on bad data rather than silently degrade |
| Newtypes | Full coverage | Every semantically distinct field gets its own type |
