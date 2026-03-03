# Domain Layer Refactoring Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Introduce a domain layer with rich types (UUIDs, newtypes, strict enums, chrono datetimes) between the DB and GraphQL/service layers, routing all reads and writes through the service.

**Architecture:** New `server/domain/` crate owns canonical types. DB layer has `FromRow` DAOs with `TryFrom`/`From` conversions. Service handles all reads+writes. GraphQL wraps domain types and calls service methods. No layer depends upward.

**Tech Stack:** Rust, sqlx (FromRow derive), juniper 0.17, chrono (DateTime<Utc>), uuid, Bazel + Gazelle

---

### Task 1: Create the domain crate with newtypes and enums

**Files:**
- Create: `server/domain/src/lib.rs`

**Step 1: Create the domain crate source file**

```rust
use chrono::{DateTime, Utc};
use uuid::Uuid;

// === ID types ===

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildScanId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskId(pub Uuid);

// === String-wrapped value types ===

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildToolType(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildToolVersion(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginVersion(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hostname(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsName(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsVersion(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JvmVendor(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JvmVersion(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskPath(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassName(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheKey(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestedTask(pub String);

// === Numeric value types ===

/// Epoch milliseconds — a point in time
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timestamp(pub i64);

/// Duration in milliseconds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Duration(pub i64);

// === Enums (strict, no Unknown fallback) ===

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildOutcome {
    Success,
    Failed,
    ParseError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskOutcome {
    UpToDate,
    Skipped,
    Failed,
    Success,
    FromCache,
    NoSource,
    AvoidedForUnknownReason,
}

// === Aggregates ===

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

**Step 2: Create the BUILD.bazel for the domain crate**

Create `server/domain/src/BUILD.bazel`:

```python
load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
    name = "domain",
    srcs = ["lib.rs"],
    visibility = ["//visibility:public"],
    deps = [
        "@crates//:chrono",
        "@crates//:uuid",
    ],
)
```

**Step 3: Verify it builds**

Run: `aspect build //server/domain/src:domain`
Expected: BUILD SUCCESS

**Step 4: Commit**

```bash
git add server/domain/src/lib.rs server/domain/src/BUILD.bazel
git commit -m "feat: add domain crate with newtypes, enums, and aggregates"
```

---

### Task 2: Add FromRow derive to DB DAOs and add domain conversions

**Files:**
- Modify: `server/db/src/lib.rs`
- Modify: `server/db/src/BUILD.bazel`

**Step 1: Add `#[derive(sqlx::FromRow)]` to `BuildScanRow` and `TaskRow`**

In `server/db/src/lib.rs`, add the derive attribute to both structs:

```rust
#[derive(sqlx::FromRow)]
pub struct BuildScanRow {
    // ... all fields unchanged
}

#[derive(sqlx::FromRow)]
pub struct TaskRow {
    // ... all fields unchanged
}
```

**Step 2: Replace manual `map_build_scan_row` and `map_task_row` with `query_as`**

Replace all `sqlx::query(...).map(map_build_scan_row)` with `sqlx::query_as::<_, BuildScanRow>(...)` throughout the file. Similarly for tasks.

Delete the `map_build_scan_row` function (lines 164-181) and `map_task_row` function (lines 231-245).

For `list_build_scans`, `get_build_scan`, `list_tasks`, and `get_task`, switch from:
```rust
sqlx::query("SELECT ...").map(map_build_scan_row).fetch_all(pool)
```
to:
```rust
sqlx::query_as::<_, BuildScanRow>("SELECT ...").fetch_all(pool)
```

**Step 3: Add `TryFrom<BuildScanRow> for domain::BuildScan` conversion**

Add to `server/db/src/lib.rs`:

```rust
use chrono::{NaiveDateTime, TimeZone, Utc};

impl TryFrom<BuildScanRow> for domain::BuildScan {
    type Error = ConversionError;

    fn try_from(row: BuildScanRow) -> Result<Self, Self::Error> {
        let id = domain::BuildScanId(
            uuid::Uuid::parse_str(&row.id)
                .map_err(|e| ConversionError(format!("invalid build scan id: {e}")))?,
        );

        let started_at = row
            .started_at
            .map(|s| parse_datetime(&s))
            .transpose()?;

        let finished_at = row
            .finished_at
            .map(|s| parse_datetime(&s))
            .transpose()?;

        let outcome = row
            .outcome
            .map(|s| parse_build_outcome(&s))
            .transpose()?;

        let requested_tasks = row
            .requested_tasks
            .map(|s| -> Result<Vec<domain::RequestedTask>, ConversionError> {
                let strings: Vec<String> = serde_json::from_str(&s)
                    .map_err(|e| ConversionError(format!("invalid requested_tasks JSON: {e}")))?;
                Ok(strings.into_iter().map(domain::RequestedTask).collect())
            })
            .transpose()?;

        let created_at = parse_datetime(&row.created_at)?;

        Ok(domain::BuildScan {
            id,
            build_tool_type: domain::BuildToolType(row.build_tool_type),
            build_tool_version: domain::BuildToolVersion(row.build_tool_version),
            plugin_version: domain::PluginVersion(row.plugin_version),
            started_at,
            finished_at,
            outcome,
            requested_tasks,
            hostname: row.hostname.map(domain::Hostname),
            os_name: row.os_name.map(domain::OsName),
            os_version: row.os_version.map(domain::OsVersion),
            jvm_vendor: row.jvm_vendor.map(domain::JvmVendor),
            jvm_version: row.jvm_version.map(domain::JvmVersion),
            created_at,
        })
    }
}
```

**Step 4: Add `TryFrom<TaskRow> for domain::Task` conversion**

```rust
impl TryFrom<TaskRow> for domain::Task {
    type Error = ConversionError;

    fn try_from(row: TaskRow) -> Result<Self, Self::Error> {
        let id = domain::TaskId(
            uuid::Uuid::parse_str(&row.id)
                .map_err(|e| ConversionError(format!("invalid task id: {e}")))?,
        );
        let scan_id = domain::BuildScanId(
            uuid::Uuid::parse_str(&row.scan_id)
                .map_err(|e| ConversionError(format!("invalid scan_id: {e}")))?,
        );
        let outcome = row
            .outcome
            .map(|s| parse_task_outcome(&s))
            .transpose()?;

        Ok(domain::Task {
            id,
            scan_id,
            task_path: domain::TaskPath(row.task_path),
            class_name: row.class_name.map(domain::ClassName),
            outcome,
            cacheable: row.cacheable,
            start_timestamp: row.start_timestamp.map(domain::Timestamp),
            finish_timestamp: row.finish_timestamp.map(domain::Timestamp),
            cache_key: row.cache_key.map(domain::CacheKey),
            origin_execution_time: row.origin_execution_time.map(domain::Duration),
        })
    }
}
```

**Step 5: Add helper functions and error type**

```rust
#[derive(Debug)]
pub struct ConversionError(pub String);

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ConversionError {}

fn parse_datetime(s: &str) -> Result<chrono::DateTime<Utc>, ConversionError> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .map(|dt| Utc.from_utc_datetime(&dt))
        .map_err(|e| ConversionError(format!("invalid datetime '{s}': {e}")))
}

fn parse_build_outcome(s: &str) -> Result<domain::BuildOutcome, ConversionError> {
    match s {
        "success" => Ok(domain::BuildOutcome::Success),
        "failed" => Ok(domain::BuildOutcome::Failed),
        "parse_error" => Ok(domain::BuildOutcome::ParseError),
        other => Err(ConversionError(format!("unknown build outcome: '{other}'"))),
    }
}

fn parse_task_outcome(s: &str) -> Result<domain::TaskOutcome, ConversionError> {
    match s {
        "UpToDate" => Ok(domain::TaskOutcome::UpToDate),
        "Skipped" => Ok(domain::TaskOutcome::Skipped),
        "Failed" => Ok(domain::TaskOutcome::Failed),
        "Success" => Ok(domain::TaskOutcome::Success),
        "FromCache" => Ok(domain::TaskOutcome::FromCache),
        "NoSource" => Ok(domain::TaskOutcome::NoSource),
        "AvoidedForUnknownReason" => Ok(domain::TaskOutcome::AvoidedForUnknownReason),
        other => Err(ConversionError(format!("unknown task outcome: '{other}'"))),
    }
}
```

**Step 6: Update `server/db/src/BUILD.bazel` to add domain + chrono deps**

Add `//server/domain/src:domain`, `@crates//:chrono`, and `@crates//:serde_json` to the deps list.

**Step 7: Verify it builds**

Run: `aspect build //server/db/src:db`
Expected: BUILD SUCCESS

**Step 8: Commit**

```bash
git add server/db/src/lib.rs server/db/src/BUILD.bazel
git commit -m "refactor: add sqlx FromRow derive to DAOs and domain conversions"
```

---

### Task 3: Expand the service layer with read methods

**Files:**
- Modify: `server/service/src/lib.rs`
- Modify: `server/service/src/BUILD.bazel`

**Step 1: Add read methods to `BuildScanService`**

Add these methods to the `impl BuildScanService` block in `server/service/src/lib.rs`:

```rust
use domain::{BuildScan, Task};

impl BuildScanService {
    // ... existing new() and process_upload() ...

    pub async fn get_build_scan(&self, id: &str) -> Result<Option<BuildScan>, Box<dyn std::error::Error>> {
        let row = db::get_build_scan(&self.pool, id).await?;
        row.map(BuildScan::try_from).transpose().map_err(|e| e.into())
    }

    pub async fn list_build_scans(
        &self,
        limit: i64,
        after_created_at: Option<&str>,
        after_id: Option<&str>,
    ) -> Result<Vec<BuildScan>, Box<dyn std::error::Error>> {
        let rows = db::list_build_scans(&self.pool, limit, after_created_at, after_id).await?;
        rows.into_iter()
            .map(BuildScan::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.into())
    }

    pub async fn get_task(&self, id: &str) -> Result<Option<Task>, Box<dyn std::error::Error>> {
        let row = db::get_task(&self.pool, id).await?;
        row.map(Task::try_from).transpose().map_err(|e| e.into())
    }

    pub async fn list_tasks(
        &self,
        scan_id: &str,
        limit: i64,
        after_id: Option<&str>,
    ) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
        let rows = db::list_tasks(&self.pool, scan_id, limit, after_id).await?;
        rows.into_iter()
            .map(Task::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.into())
    }

    pub async fn count_tasks(&self, scan_id: &str) -> Result<i64, Box<dyn std::error::Error>> {
        Ok(db::count_tasks(&self.pool, scan_id).await?)
    }

    pub async fn count_build_scans(&self) -> Result<i64, Box<dyn std::error::Error>> {
        Ok(db::count_build_scans(&self.pool).await?)
    }
}
```

**Step 2: Update the write path (`process_upload`) to use domain types for outcome**

Update the outcome derivation to use `domain::BuildOutcome` internally and convert to string for DB storage. The task outcome conversion (`format!("{:?}", o)`) already produces the string format that `parse_task_outcome` expects (e.g. `"Success"`, `"Failed"`), so this stays compatible.

**Step 3: Update `server/service/src/BUILD.bazel`**

Add `//server/domain/src:domain` to deps.

**Step 4: Verify it builds**

Run: `aspect build //server/service/src:service`
Expected: BUILD SUCCESS

**Step 5: Commit**

```bash
git add server/service/src/lib.rs server/service/src/BUILD.bazel
git commit -m "feat: expand service layer with read methods returning domain types"
```

---

### Task 4: Refactor GraphQL to wrap domain types and call service

**Files:**
- Modify: `server/graphql/src/lib.rs`
- Modify: `server/graphql/src/BUILD.bazel`

**Step 1: Change `Context` to hold `Arc<BuildScanService>` instead of `Arc<SqlitePool>`**

In `server/graphql/src/lib.rs`, change:

```rust
pub struct Context {
    pub service: Arc<service::BuildScanService>,
}
```

**Step 2: Change `BuildScan` to wrap `domain::BuildScan`**

Replace:
```rust
pub struct BuildScan {
    pub row: db::BuildScanRow,
}
```
with:
```rust
pub struct BuildScan {
    pub scan: domain::BuildScan,
}
```

Update all resolver methods to read from `self.scan` instead of `self.row`, using the newtype inner values. For example:

```rust
#[graphql_object(context = Context, impl = NodeValue)]
impl BuildScan {
    fn id(&self) -> ID {
        RelayId::encode("BuildScan", &self.scan.id.0.to_string())
    }

    fn scan_id(&self) -> String {
        self.scan.id.0.to_string()
    }

    fn build_tool_type(&self) -> &str {
        &self.scan.build_tool_type.0
    }

    fn build_tool_version(&self) -> &str {
        &self.scan.build_tool_version.0
    }

    fn plugin_version(&self) -> &str {
        &self.scan.plugin_version.0
    }

    fn started_at(&self) -> Option<String> {
        self.scan.started_at.map(|dt| dt.to_rfc3339())
    }

    fn finished_at(&self) -> Option<String> {
        self.scan.finished_at.map(|dt| dt.to_rfc3339())
    }

    fn outcome(&self) -> Option<&str> {
        self.scan.outcome.as_ref().map(|o| match o {
            domain::BuildOutcome::Success => "success",
            domain::BuildOutcome::Failed => "failed",
            domain::BuildOutcome::ParseError => "parse_error",
        })
    }

    fn hostname(&self) -> Option<&str> {
        self.scan.hostname.as_ref().map(|h| h.0.as_str())
    }

    fn os_name(&self) -> Option<&str> {
        self.scan.os_name.as_ref().map(|n| n.0.as_str())
    }

    fn os_version(&self) -> Option<&str> {
        self.scan.os_version.as_ref().map(|v| v.0.as_str())
    }

    fn jvm_vendor(&self) -> Option<&str> {
        self.scan.jvm_vendor.as_ref().map(|v| v.0.as_str())
    }

    fn jvm_version(&self) -> Option<&str> {
        self.scan.jvm_version.as_ref().map(|v| v.0.as_str())
    }

    fn created_at(&self) -> String {
        self.scan.created_at.to_rfc3339()
    }

    fn requested_tasks(&self) -> Vec<String> {
        self.scan
            .requested_tasks
            .as_ref()
            .map(|tasks| tasks.iter().map(|t| t.0.clone()).collect())
            .unwrap_or_default()
    }

    async fn task_count(&self, context: &Context) -> FieldResult<i32> {
        let count = context
            .service
            .count_tasks(&self.scan.id.0.to_string())
            .await
            .map_err(|e| FieldError::from(e.to_string()))? as i32;
        Ok(count)
    }

    async fn tasks(
        &self,
        context: &Context,
        first: Option<i32>,
        after: Option<String>,
    ) -> FieldResult<TaskConnection> {
        let limit = validate_pagination(first).map_err(FieldError::from)?;
        let after_id = after
            .as_deref()
            .map(Cursor::decode)
            .transpose()
            .map_err(FieldError::from)?
            .map(|c| c.value);

        let scan_id_str = self.scan.id.0.to_string();
        let mut tasks = context
            .service
            .list_tasks(&scan_id_str, (limit + 1) as i64, after_id.as_deref())
            .await
            .map_err(|e| FieldError::from(e.to_string()))?;

        let has_next_page = tasks.len() > limit as usize;
        if has_next_page {
            tasks.pop();
        }

        let end_cursor = tasks
            .last()
            .map(|t| Cursor::new(t.id.0.to_string()).encode());

        let edges: Vec<TaskEdge> = tasks
            .into_iter()
            .map(|t| {
                let cursor = Cursor::new(t.id.0.to_string()).encode();
                TaskEdge {
                    cursor,
                    node: Task { task: t },
                }
            })
            .collect();

        let total_count = context
            .service
            .count_tasks(&scan_id_str)
            .await
            .map_err(|e| FieldError::from(e.to_string()))? as i32;

        Ok(TaskConnection {
            edges,
            page_info: PageInfo {
                has_next_page,
                end_cursor,
            },
            total_count,
        })
    }
}
```

**Step 3: Change `Task` to wrap `domain::Task`**

Replace:
```rust
pub struct Task {
    pub row: db::TaskRow,
}
```
with:
```rust
pub struct Task {
    pub task: domain::Task,
}
```

Update all resolver methods:

```rust
#[graphql_object(context = Context, impl = NodeValue)]
impl Task {
    fn id(&self) -> ID {
        RelayId::encode("Task", &self.task.id.0.to_string())
    }

    fn task_id(&self) -> String {
        self.task.id.0.to_string()
    }

    fn scan_id(&self) -> String {
        self.task.scan_id.0.to_string()
    }

    fn task_path(&self) -> &str {
        &self.task.task_path.0
    }

    fn class_name(&self) -> Option<&str> {
        self.task.class_name.as_ref().map(|c| c.0.as_str())
    }

    fn outcome(&self) -> Option<&str> {
        self.task.outcome.as_ref().map(|o| match o {
            domain::TaskOutcome::UpToDate => "UpToDate",
            domain::TaskOutcome::Skipped => "Skipped",
            domain::TaskOutcome::Failed => "Failed",
            domain::TaskOutcome::Success => "Success",
            domain::TaskOutcome::FromCache => "FromCache",
            domain::TaskOutcome::NoSource => "NoSource",
            domain::TaskOutcome::AvoidedForUnknownReason => "AvoidedForUnknownReason",
        })
    }

    fn cacheable(&self) -> Option<bool> {
        self.task.cacheable
    }

    fn start_timestamp(&self) -> Option<f64> {
        self.task.start_timestamp.map(|t| t.0 as f64)
    }

    fn finish_timestamp(&self) -> Option<f64> {
        self.task.finish_timestamp.map(|t| t.0 as f64)
    }

    fn duration_ms(&self) -> Option<f64> {
        match (self.task.start_timestamp, self.task.finish_timestamp) {
            (Some(start), Some(finish)) => Some((finish.0 - start.0) as f64),
            _ => None,
        }
    }

    fn cache_key(&self) -> Option<&str> {
        self.task.cache_key.as_ref().map(|k| k.0.as_str())
    }

    fn origin_execution_time(&self) -> Option<f64> {
        self.task.origin_execution_time.map(|d| d.0 as f64)
    }
}
```

**Step 4: Update `QueryRoot` resolvers to call service**

Replace all `db::*(&context.pool, ...)` calls with `context.service.*` calls. The `node()` resolver becomes:

```rust
async fn node(context: &Context, id: ID) -> FieldResult<Option<NodeValue>> {
    let relay_id = RelayId::decode(&id).map_err(FieldError::from)?;
    match relay_id.type_name.as_str() {
        "BuildScan" => {
            let scan = context.service.get_build_scan(&relay_id.raw_id)
                .await
                .map_err(|e| FieldError::from(e.to_string()))?;
            Ok(scan.map(|s| NodeValue::BuildScan(BuildScan { scan: s })))
        }
        "Task" => {
            let task = context.service.get_task(&relay_id.raw_id)
                .await
                .map_err(|e| FieldError::from(e.to_string()))?;
            Ok(task.map(|t| NodeValue::Task(Task { task: t })))
        }
        _ => Ok(None),
    }
}
```

The `build_scans()` resolver needs to handle cursors. Note: cursors currently encode `{created_at, id}` as a JSON string for composite pagination. Since the service `list_build_scans` still accepts raw string parameters for cursor values, the cursor decoding stays in the GraphQL layer (it's a presentation concern):

```rust
async fn build_scans(
    context: &Context,
    first: Option<i32>,
    after: Option<String>,
) -> FieldResult<BuildScanConnection> {
    let limit = validate_pagination(first).map_err(FieldError::from)?;
    let cursor = after
        .as_deref()
        .map(Cursor::decode)
        .transpose()
        .map_err(FieldError::from)?;
    let (after_created_at, after_id) = if let Some(c) = cursor {
        let v: serde_json::Value = serde_json::from_str(&c.value)
            .map_err(|_| FieldError::from("Invalid cursor format".to_string()))?;
        let created_at = v["created_at"]
            .as_str()
            .ok_or_else(|| FieldError::from("Missing created_at in cursor".to_string()))?
            .to_string();
        let id = v["id"]
            .as_str()
            .ok_or_else(|| FieldError::from("Missing id in cursor".to_string()))?
            .to_string();
        (Some(created_at), Some(id))
    } else {
        (None, None)
    };

    let mut scans = context
        .service
        .list_build_scans(
            (limit + 1) as i64,
            after_created_at.as_deref(),
            after_id.as_deref(),
        )
        .await
        .map_err(|e| FieldError::from(e.to_string()))?;

    let has_next_page = scans.len() > limit as usize;
    if has_next_page {
        scans.pop();
    }

    let end_cursor = scans.last().map(|s| encode_build_scan_cursor(s));

    let edges: Vec<BuildScanEdge> = scans
        .into_iter()
        .map(|s| {
            let cursor = encode_build_scan_cursor(&s);
            BuildScanEdge {
                cursor,
                node: BuildScan { scan: s },
            }
        })
        .collect();

    let total_count = context
        .service
        .count_build_scans()
        .await
        .map_err(|e| FieldError::from(e.to_string()))? as i32;

    Ok(BuildScanConnection {
        edges,
        page_info: PageInfo {
            has_next_page,
            end_cursor,
        },
        total_count,
    })
}
```

**Step 5: Update `encode_build_scan_cursor` to work with domain type**

```rust
fn encode_build_scan_cursor(scan: &domain::BuildScan) -> String {
    let value = serde_json::json!({
        "created_at": scan.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        "id": scan.id.0.to_string()
    })
    .to_string();
    Cursor::new(value).encode()
}
```

**Step 6: Update `build_scan()` resolver**

```rust
async fn build_scan(context: &Context, id: ID) -> FieldResult<Option<BuildScan>> {
    let raw_id = if let Ok(relay_id) = RelayId::decode(&id) {
        relay_id.raw_id
    } else {
        id.to_string()
    };

    let scan = context
        .service
        .get_build_scan(&raw_id)
        .await
        .map_err(|e| FieldError::from(e.to_string()))?;

    Ok(scan.map(|s| BuildScan { scan: s }))
}
```

**Step 7: Update `server/graphql/src/BUILD.bazel`**

Replace `//server/db/src:db` with `//server/domain/src:domain` and `//server/service/src:service`. Add `@crates//:chrono`. Remove `@crates//:sqlx` (no longer needed directly).

```python
rust_library(
    name = "graphql",
    srcs = ["lib.rs"],
    visibility = ["//visibility:public"],
    deps = [
        "//server/domain/src:domain",
        "//server/graphql/relay/src:relay",
        "//server/service/src:service",
        "@crates//:axum",
        "@crates//:chrono",
        "@crates//:juniper",
        "@crates//:juniper_axum",
        "@crates//:serde_json",
    ],
)
```

**Step 8: Verify it builds**

Run: `aspect build //server/graphql/src:graphql`
Expected: BUILD SUCCESS

**Step 9: Commit**

```bash
git add server/graphql/src/lib.rs server/graphql/src/BUILD.bazel
git commit -m "refactor: graphql wraps domain types and calls service layer"
```

---

### Task 5: Update main.rs to wire service into GraphQL context

**Files:**
- Modify: `server/src/main.rs`

**Step 1: Change GraphQL context creation**

In `server/src/main.rs`, replace lines 44-47:

```rust
let schema = Arc::new(graphql::create_schema());
let context = Arc::new(graphql::Context {
    pool: Arc::new(pool.clone()),
});
```

with:

```rust
let schema = Arc::new(graphql::create_schema());
let context = Arc::new(graphql::Context {
    service: service.clone(),
});
```

The `service` variable (line 42) is already `Arc<BuildScanService>`, so this works directly.

**Step 2: Remove the extra `pool.clone()` since GraphQL no longer needs it**

The `pool` is only needed by `db::connect` and `BuildScanService::new`. Remove `pool.clone()` from the context line.

**Step 3: Verify it builds**

Run: `aspect build //server/src:main`
Expected: BUILD SUCCESS

**Step 4: Commit**

```bash
git add server/src/main.rs
git commit -m "refactor: wire service into graphql context instead of pool"
```

---

### Task 6: Run Gazelle, format, and full test suite

**Step 1: Regenerate BUILD files**

Run: `bazel run gazelle`

Note: Gazelle may update BUILD.bazel files with auto-detected deps. Review changes and keep them.

**Step 2: Format all files**

Run: `bazel run //tools/format`

**Step 3: Run all tests**

Run: `aspect test //...`
Expected: All tests pass

**Step 4: Run a full build**

Run: `aspect build //...`
Expected: BUILD SUCCESS

**Step 5: Commit any formatting/Gazelle changes**

```bash
git add -A
git commit -m "chore: run gazelle and format"
```

---

### Task 7: Smoke test with a real upload

**Step 1: Start the server**

Run: `aspect run //server/src:main`

**Step 2: Verify GraphiQL loads**

Open `http://localhost:3000/graphiql` in a browser. Confirm the schema loads and shows `BuildScan` fields.

**Step 3: Run a Gradle build against the server (if available)**

If a test Gradle project is available, run a build scan upload and verify:
- The upload succeeds
- `build_scans` query returns the scan with correct field types
- `tasks` nested query returns tasks with outcomes

This is a manual verification step — not automated.
