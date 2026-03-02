use chrono::{NaiveDateTime, TimeZone, Utc};
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions, sqlite::SqlitePoolOptions};
use std::str::FromStr;
use uuid::Uuid;

pub async fn connect(url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(url)?.pragma("foreign_keys", "ON");
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    migrations::MIGRATOR
        .run(&pool)
        .await
        .map_err(|e| sqlx::Error::Configuration(e.into()))?;

    Ok(pool)
}

#[derive(Debug, sqlx::FromRow)]
pub struct BuildScanRow {
    pub id: String,
    pub build_tool_type: String,
    pub build_tool_version: String,
    pub plugin_version: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub outcome: Option<String>,
    pub requested_tasks: Option<String>,
    pub hostname: Option<String>,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub jvm_vendor: Option<String>,
    pub jvm_version: Option<String>,
    pub created_at: String,
}

#[derive(Debug, sqlx::FromRow)]
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

impl TryFrom<BuildScanRow> for domain::BuildScan {
    type Error = ConversionError;

    fn try_from(row: BuildScanRow) -> Result<Self, Self::Error> {
        let id = domain::BuildScanId(
            uuid::Uuid::parse_str(&row.id)
                .map_err(|e| ConversionError(format!("invalid build scan id: {e}")))?,
        );

        let started_at = row.started_at.map(|s| parse_datetime(&s)).transpose()?;
        let finished_at = row.finished_at.map(|s| parse_datetime(&s)).transpose()?;

        let outcome = row
            .outcome
            .map(|s| {
                s.parse::<domain::BuildOutcome>()
                    .map_err(ConversionError)
            })
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
            .map(|s| {
                s.parse::<domain::TaskOutcome>()
                    .map_err(ConversionError)
            })
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

#[allow(clippy::too_many_arguments)]
pub async fn insert_build_scan<'c, E: sqlx::Executor<'c, Database = sqlx::Sqlite>>(
    executor: E,
    id: &str,
    build_tool_type: &str,
    build_tool_version: &str,
    plugin_version: &str,
    started_at: Option<&str>,
    finished_at: Option<&str>,
    outcome: Option<&str>,
    requested_tasks: Option<&str>,
    hostname: Option<&str>,
    os_name: Option<&str>,
    os_version: Option<&str>,
    jvm_vendor: Option<&str>,
    jvm_version: Option<&str>,
    raw_payload: Option<&[u8]>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO build_scans (id, build_tool_type, build_tool_version, plugin_version, started_at, finished_at, outcome, requested_tasks, hostname, os_name, os_version, jvm_vendor, jvm_version, raw_payload) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(build_tool_type)
    .bind(build_tool_version)
    .bind(plugin_version)
    .bind(started_at)
    .bind(finished_at)
    .bind(outcome)
    .bind(requested_tasks)
    .bind(hostname)
    .bind(os_name)
    .bind(os_version)
    .bind(jvm_vendor)
    .bind(jvm_version)
    .bind(raw_payload)
    .execute(executor)
    .await?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_task<'c, E: sqlx::Executor<'c, Database = sqlx::Sqlite>>(
    executor: E,
    scan_id: &str,
    task_path: &str,
    class_name: Option<&str>,
    outcome: Option<&str>,
    cacheable: Option<bool>,
    start_timestamp: Option<i64>,
    finish_timestamp: Option<i64>,
    cache_key: Option<&str>,
    origin_execution_time: Option<i64>,
) -> Result<(), sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let cacheable_int: Option<i64> = cacheable.map(|b| if b { 1 } else { 0 });

    sqlx::query(
        "INSERT INTO tasks (id, scan_id, task_path, class_name, outcome, cacheable, start_timestamp, finish_timestamp, cache_key, origin_execution_time) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(scan_id)
    .bind(task_path)
    .bind(class_name)
    .bind(outcome)
    .bind(cacheable_int)
    .bind(start_timestamp)
    .bind(finish_timestamp)
    .bind(cache_key)
    .bind(origin_execution_time)
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn list_build_scans(
    pool: &SqlitePool,
    limit: i64,
    after_created_at: Option<&str>,
    after_id: Option<&str>,
) -> Result<Vec<BuildScanRow>, sqlx::Error> {
    let rows = match (after_created_at, after_id) {
        (Some(cursor_created_at), Some(cursor_id)) => {
            sqlx::query_as::<_, BuildScanRow>(
                "SELECT id, build_tool_type, build_tool_version, plugin_version, started_at, finished_at, outcome, requested_tasks, hostname, os_name, os_version, jvm_vendor, jvm_version, created_at \
                 FROM build_scans WHERE (created_at, id) < (?1, ?2) ORDER BY created_at DESC, id DESC LIMIT ?3",
            )
            .bind(cursor_created_at)
            .bind(cursor_id)
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
        _ => {
            sqlx::query_as::<_, BuildScanRow>(
                "SELECT id, build_tool_type, build_tool_version, plugin_version, started_at, finished_at, outcome, requested_tasks, hostname, os_name, os_version, jvm_vendor, jvm_version, created_at \
                 FROM build_scans ORDER BY created_at DESC, id DESC LIMIT ?",
            )
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
    };

    Ok(rows)
}

pub async fn get_build_scan(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<BuildScanRow>, sqlx::Error> {
    let row = sqlx::query_as::<_, BuildScanRow>(
        "SELECT id, build_tool_type, build_tool_version, plugin_version, started_at, finished_at, outcome, requested_tasks, hostname, os_name, os_version, jvm_vendor, jvm_version, created_at \
         FROM build_scans WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn list_tasks(
    pool: &SqlitePool,
    scan_id: &str,
    limit: i64,
    after_id: Option<&str>,
) -> Result<Vec<TaskRow>, sqlx::Error> {
    let rows = if let Some(cursor) = after_id {
        sqlx::query_as::<_, TaskRow>(
            "SELECT id, scan_id, task_path, class_name, outcome, cacheable, start_timestamp, finish_timestamp, cache_key, origin_execution_time \
             FROM tasks WHERE scan_id = ? AND id > ? ORDER BY id LIMIT ?",
        )
        .bind(scan_id)
        .bind(cursor)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, TaskRow>(
            "SELECT id, scan_id, task_path, class_name, outcome, cacheable, start_timestamp, finish_timestamp, cache_key, origin_execution_time \
             FROM tasks WHERE scan_id = ? ORDER BY id LIMIT ?",
        )
        .bind(scan_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };

    Ok(rows)
}

pub async fn count_tasks(pool: &SqlitePool, scan_id: &str) -> Result<i64, sqlx::Error> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE scan_id = ?")
        .bind(scan_id)
        .fetch_one(pool)
        .await?;

    Ok(count)
}

pub async fn count_build_scans(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM build_scans")
        .fetch_one(pool)
        .await?;

    Ok(count)
}

pub async fn get_task(pool: &SqlitePool, id: &str) -> Result<Option<TaskRow>, sqlx::Error> {
    let row = sqlx::query_as::<_, TaskRow>(
        "SELECT id, scan_id, task_path, class_name, outcome, cacheable, start_timestamp, finish_timestamp, cache_key, origin_execution_time \
         FROM tasks WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}
