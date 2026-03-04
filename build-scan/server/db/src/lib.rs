use anyhow::{Context, Result};
use chrono::{NaiveDateTime, TimeZone, Utc};
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions, sqlite::SqlitePoolOptions};
use std::str::FromStr;

pub async fn connect(url: &str) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(url)?.pragma("foreign_keys", "ON");
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    migrations::MIGRATOR
        .run(&pool)
        .await
        .context("failed to run migrations")?;

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

fn parse_datetime(s: &str) -> Result<chrono::DateTime<Utc>> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .map(|dt| Utc.from_utc_datetime(&dt))
        .with_context(|| format!("invalid datetime '{s}'"))
}

impl TryFrom<BuildScanRow> for domain::BuildScan {
    type Error = anyhow::Error;

    fn try_from(row: BuildScanRow) -> Result<Self, Self::Error> {
        let id = domain::BuildScanId(
            uuid::Uuid::parse_str(&row.id)
                .with_context(|| format!("invalid build scan id '{}'", row.id))?,
        );

        let started_at = row.started_at.map(|s| parse_datetime(&s)).transpose()?;
        let finished_at = row.finished_at.map(|s| parse_datetime(&s)).transpose()?;

        let outcome = row
            .outcome
            .map(|s| {
                s.parse::<domain::BuildOutcome>()
                    .map_err(anyhow::Error::msg)
            })
            .transpose()?;

        let requested_tasks = row
            .requested_tasks
            .map(|s| -> Result<Vec<domain::RequestedTask>> {
                let strings: Vec<String> =
                    serde_json::from_str(&s).context("invalid requested_tasks JSON")?;
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
    type Error = anyhow::Error;

    fn try_from(row: TaskRow) -> Result<Self, Self::Error> {
        let id = domain::TaskId(
            uuid::Uuid::parse_str(&row.id)
                .with_context(|| format!("invalid task id '{}'", row.id))?,
        );
        let scan_id = domain::BuildScanId(
            uuid::Uuid::parse_str(&row.scan_id)
                .with_context(|| format!("invalid scan_id '{}'", row.scan_id))?,
        );
        let outcome = row
            .outcome
            .map(|s| s.parse::<domain::TaskOutcome>().map_err(anyhow::Error::msg))
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

pub async fn insert_build_scan<'c, E: sqlx::Executor<'c, Database = sqlx::Sqlite>>(
    executor: E,
    scan: &domain::BuildScan,
    raw_payload: Option<&[u8]>,
) -> Result<()> {
    let id = scan.id.0.to_string();
    let started_at = scan
        .started_at
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string());
    let finished_at = scan
        .finished_at
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string());
    let outcome = scan.outcome.map(|o| o.to_string());
    let requested_tasks = scan.requested_tasks.as_ref().map(|tasks| {
        serde_json::to_string(&tasks.iter().map(|t| &t.0).collect::<Vec<_>>()).unwrap()
    });

    sqlx::query(
        "INSERT INTO build_scans (id, build_tool_type, build_tool_version, plugin_version, started_at, finished_at, outcome, requested_tasks, hostname, os_name, os_version, jvm_vendor, jvm_version, raw_payload) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&scan.build_tool_type.0)
    .bind(&scan.build_tool_version.0)
    .bind(&scan.plugin_version.0)
    .bind(started_at.as_deref())
    .bind(finished_at.as_deref())
    .bind(outcome.as_deref())
    .bind(requested_tasks.as_deref())
    .bind(scan.hostname.as_ref().map(|h| &h.0))
    .bind(scan.os_name.as_ref().map(|n| &n.0))
    .bind(scan.os_version.as_ref().map(|v| &v.0))
    .bind(scan.jvm_vendor.as_ref().map(|v| &v.0))
    .bind(scan.jvm_version.as_ref().map(|v| &v.0))
    .bind(raw_payload)
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn insert_task<'c, E: sqlx::Executor<'c, Database = sqlx::Sqlite>>(
    executor: E,
    task: &domain::Task,
) -> Result<()> {
    let id = task.id.0.to_string();
    let scan_id = task.scan_id.0.to_string();
    let outcome = task.outcome.map(|o| o.to_string());
    let cacheable_int: Option<i64> = task.cacheable.map(|b| if b { 1 } else { 0 });

    sqlx::query(
        "INSERT INTO tasks (id, scan_id, task_path, class_name, outcome, cacheable, start_timestamp, finish_timestamp, cache_key, origin_execution_time) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&scan_id)
    .bind(&task.task_path.0)
    .bind(task.class_name.as_ref().map(|c| &c.0))
    .bind(outcome.as_deref())
    .bind(cacheable_int)
    .bind(task.start_timestamp.map(|t| t.0))
    .bind(task.finish_timestamp.map(|t| t.0))
    .bind(task.cache_key.as_ref().map(|k| &k.0))
    .bind(task.origin_execution_time.map(|d| d.0))
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn list_build_scans(
    pool: &SqlitePool,
    limit: i64,
    after_created_at: Option<&str>,
    after_id: Option<&str>,
) -> Result<Vec<BuildScanRow>> {
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

pub async fn get_build_scan(pool: &SqlitePool, id: &str) -> Result<Option<BuildScanRow>> {
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
) -> Result<Vec<TaskRow>> {
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

pub async fn count_tasks(pool: &SqlitePool, scan_id: &str) -> Result<i64> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE scan_id = ?")
        .bind(scan_id)
        .fetch_one(pool)
        .await?;

    Ok(count)
}

pub async fn count_build_scans(pool: &SqlitePool) -> Result<i64> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM build_scans")
        .fetch_one(pool)
        .await?;

    Ok(count)
}

pub async fn get_task(pool: &SqlitePool, id: &str) -> Result<Option<TaskRow>> {
    let row = sqlx::query_as::<_, TaskRow>(
        "SELECT id, scan_id, task_path, class_name, outcome, cacheable, start_timestamp, finish_timestamp, cache_key, origin_execution_time \
         FROM tasks WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}
