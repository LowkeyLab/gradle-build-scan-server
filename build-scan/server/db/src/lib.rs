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
struct BuildScanRow {
    id: String,
    build_tool_type: String,
    build_tool_version: String,
    plugin_version: String,
    started_at: Option<String>,
    finished_at: Option<String>,
    outcome: Option<String>,
    requested_tasks: Option<String>,
    hostname: Option<String>,
    os_name: Option<String>,
    os_version: Option<String>,
    jvm_vendor: Option<String>,
    jvm_version: Option<String>,
    created_at: String,
}

#[derive(Debug, sqlx::FromRow)]
struct TaskRow {
    id: String,
    scan_id: String,
    task_path: String,
    class_name: Option<String>,
    outcome: Option<String>,
    cacheable: Option<bool>,
    start_timestamp: Option<i64>,
    finish_timestamp: Option<i64>,
    cache_key: Option<String>,
    origin_execution_time: Option<i64>,
}

#[derive(Debug, sqlx::FromRow)]
struct TestRow {
    id: String,
    scan_id: String,
    class_name: String,
    method_name: Option<String>,
    executor_name: Option<String>,
    outcome: Option<String>,
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

impl From<&domain::BuildScan> for BuildScanRow {
    fn from(scan: &domain::BuildScan) -> Self {
        let requested_tasks = scan.requested_tasks.as_ref().map(|tasks| {
            serde_json::to_string(&tasks.iter().map(|t| &t.0).collect::<Vec<_>>())
                .expect("serializing Vec<&String> should not fail")
        });

        Self {
            id: scan.id.0.to_string(),
            build_tool_type: scan.build_tool_type.0.clone(),
            build_tool_version: scan.build_tool_version.0.clone(),
            plugin_version: scan.plugin_version.0.clone(),
            started_at: scan
                .started_at
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string()),
            finished_at: scan
                .finished_at
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string()),
            outcome: scan.outcome.map(|o| o.to_string()),
            requested_tasks,
            hostname: scan.hostname.as_ref().map(|h| h.0.clone()),
            os_name: scan.os_name.as_ref().map(|n| n.0.clone()),
            os_version: scan.os_version.as_ref().map(|v| v.0.clone()),
            jvm_vendor: scan.jvm_vendor.as_ref().map(|v| v.0.clone()),
            jvm_version: scan.jvm_version.as_ref().map(|v| v.0.clone()),
            created_at: scan.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

impl From<&domain::Task> for TaskRow {
    fn from(task: &domain::Task) -> Self {
        Self {
            id: task.id.0.to_string(),
            scan_id: task.scan_id.0.to_string(),
            task_path: task.task_path.0.clone(),
            class_name: task.class_name.as_ref().map(|c| c.0.clone()),
            outcome: task.outcome.map(|o| o.to_string()),
            cacheable: task.cacheable,
            start_timestamp: task.start_timestamp.map(|t| t.0),
            finish_timestamp: task.finish_timestamp.map(|t| t.0),
            cache_key: task.cache_key.as_ref().map(|k| k.0.clone()),
            origin_execution_time: task.origin_execution_time.map(|d| d.0),
        }
    }
}

impl TryFrom<TestRow> for domain::Test {
    type Error = anyhow::Error;
    fn try_from(row: TestRow) -> Result<Self, Self::Error> {
        let id = domain::TestId(
            uuid::Uuid::parse_str(&row.id)
                .with_context(|| format!("invalid test id '{}'", row.id))?,
        );
        let scan_id = domain::BuildScanId(
            uuid::Uuid::parse_str(&row.scan_id)
                .with_context(|| format!("invalid scan_id '{}'", row.scan_id))?,
        );
        let outcome = row
            .outcome
            .map(|s| s.parse::<domain::TestOutcome>().map_err(anyhow::Error::msg))
            .transpose()?;
        Ok(domain::Test {
            id,
            scan_id,
            class_name: domain::TestClassName(row.class_name),
            method_name: row.method_name.map(domain::MethodName),
            executor_name: row.executor_name.map(domain::ExecutorName),
            outcome,
        })
    }
}

impl From<&domain::Test> for TestRow {
    fn from(test: &domain::Test) -> Self {
        Self {
            id: test.id.0.to_string(),
            scan_id: test.scan_id.0.to_string(),
            class_name: test.class_name.0.clone(),
            method_name: test.method_name.as_ref().map(|m| m.0.clone()),
            executor_name: test.executor_name.as_ref().map(|e| e.0.clone()),
            outcome: test.outcome.map(|o| o.to_string()),
        }
    }
}

pub async fn insert_build_scan<'c, E: sqlx::Executor<'c, Database = sqlx::Sqlite>>(
    executor: E,
    scan: &domain::BuildScan,
    raw_payload: Option<&[u8]>,
) -> Result<()> {
    let row = BuildScanRow::from(scan);

    sqlx::query(
        "INSERT INTO build_scans (id, build_tool_type, build_tool_version, plugin_version, started_at, finished_at, outcome, requested_tasks, hostname, os_name, os_version, jvm_vendor, jvm_version, raw_payload, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&row.id)
    .bind(&row.build_tool_type)
    .bind(&row.build_tool_version)
    .bind(&row.plugin_version)
    .bind(row.started_at.as_deref())
    .bind(row.finished_at.as_deref())
    .bind(row.outcome.as_deref())
    .bind(row.requested_tasks.as_deref())
    .bind(row.hostname.as_deref())
    .bind(row.os_name.as_deref())
    .bind(row.os_version.as_deref())
    .bind(row.jvm_vendor.as_deref())
    .bind(row.jvm_version.as_deref())
    .bind(raw_payload)
    .bind(&row.created_at)
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn insert_task<'c, E: sqlx::Executor<'c, Database = sqlx::Sqlite>>(
    executor: E,
    task: &domain::Task,
) -> Result<()> {
    let row = TaskRow::from(task);
    let cacheable_int: Option<i64> = row.cacheable.map(|b| if b { 1 } else { 0 });

    sqlx::query(
        "INSERT INTO tasks (id, scan_id, task_path, class_name, outcome, cacheable, start_timestamp, finish_timestamp, cache_key, origin_execution_time) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&row.id)
    .bind(&row.scan_id)
    .bind(&row.task_path)
    .bind(row.class_name.as_deref())
    .bind(row.outcome.as_deref())
    .bind(cacheable_int)
    .bind(row.start_timestamp)
    .bind(row.finish_timestamp)
    .bind(row.cache_key.as_deref())
    .bind(row.origin_execution_time)
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn list_build_scans(
    pool: &SqlitePool,
    limit: i64,
    after_created_at: Option<&str>,
    after_id: Option<&str>,
) -> Result<Vec<domain::BuildScan>> {
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

    rows.into_iter()
        .map(domain::BuildScan::try_from)
        .collect::<Result<Vec<_>>>()
}

pub async fn get_build_scan(pool: &SqlitePool, id: &str) -> Result<Option<domain::BuildScan>> {
    let row = sqlx::query_as::<_, BuildScanRow>(
        "SELECT id, build_tool_type, build_tool_version, plugin_version, started_at, finished_at, outcome, requested_tasks, hostname, os_name, os_version, jvm_vendor, jvm_version, created_at \
         FROM build_scans WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    row.map(domain::BuildScan::try_from).transpose()
}

pub async fn list_tasks(
    pool: &SqlitePool,
    scan_id: &str,
    limit: i64,
    after_id: Option<&str>,
) -> Result<Vec<domain::Task>> {
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

    rows.into_iter()
        .map(domain::Task::try_from)
        .collect::<Result<Vec<_>>>()
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

pub async fn get_task(pool: &SqlitePool, id: &str) -> Result<Option<domain::Task>> {
    let row = sqlx::query_as::<_, TaskRow>(
        "SELECT id, scan_id, task_path, class_name, outcome, cacheable, start_timestamp, finish_timestamp, cache_key, origin_execution_time \
         FROM tasks WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    row.map(domain::Task::try_from).transpose()
}

pub async fn insert_test<'c, E: sqlx::Executor<'c, Database = sqlx::Sqlite>>(
    executor: E,
    test: &domain::Test,
) -> Result<()> {
    let row = TestRow::from(test);
    sqlx::query(
        "INSERT INTO tests (id, scan_id, class_name, method_name, executor_name, outcome) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&row.id)
    .bind(&row.scan_id)
    .bind(&row.class_name)
    .bind(row.method_name.as_deref())
    .bind(row.executor_name.as_deref())
    .bind(row.outcome.as_deref())
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn list_tests(
    pool: &SqlitePool,
    scan_id: &str,
    limit: i64,
    after_id: Option<&str>,
) -> Result<Vec<domain::Test>> {
    let rows = if let Some(cursor) = after_id {
        sqlx::query_as::<_, TestRow>(
            "SELECT id, scan_id, class_name, method_name, executor_name, outcome \
             FROM tests WHERE scan_id = ? AND id > ? ORDER BY id LIMIT ?",
        )
        .bind(scan_id)
        .bind(cursor)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, TestRow>(
            "SELECT id, scan_id, class_name, method_name, executor_name, outcome \
             FROM tests WHERE scan_id = ? ORDER BY id LIMIT ?",
        )
        .bind(scan_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };
    rows.into_iter()
        .map(domain::Test::try_from)
        .collect::<Result<Vec<_>>>()
}

pub async fn count_tests(pool: &SqlitePool, scan_id: &str) -> Result<i64> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tests WHERE scan_id = ?")
        .bind(scan_id)
        .fetch_one(pool)
        .await?;
    Ok(count)
}

pub async fn get_test(pool: &SqlitePool, id: &str) -> Result<Option<domain::Test>> {
    let row = sqlx::query_as::<_, TestRow>(
        "SELECT id, scan_id, class_name, method_name, executor_name, outcome \
         FROM tests WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    row.map(domain::Test::try_from).transpose()
}
