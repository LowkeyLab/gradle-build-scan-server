use sqlx::{
    Row, SqlitePool, sqlite::SqliteConnectOptions, sqlite::SqlitePoolOptions, sqlite::SqliteRow,
};
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
            sqlx::query(
                "SELECT id, build_tool_type, build_tool_version, plugin_version, started_at, finished_at, outcome, requested_tasks, hostname, os_name, os_version, jvm_vendor, jvm_version, created_at \
                 FROM build_scans WHERE (created_at, id) < (?1, ?2) ORDER BY created_at DESC, id DESC LIMIT ?3",
            )
            .bind(cursor_created_at)
            .bind(cursor_id)
            .bind(limit)
            .map(map_build_scan_row)
            .fetch_all(pool)
            .await?
        }
        _ => {
            sqlx::query(
                "SELECT id, build_tool_type, build_tool_version, plugin_version, started_at, finished_at, outcome, requested_tasks, hostname, os_name, os_version, jvm_vendor, jvm_version, created_at \
                 FROM build_scans ORDER BY created_at DESC, id DESC LIMIT ?",
            )
            .bind(limit)
            .map(map_build_scan_row)
            .fetch_all(pool)
            .await?
        }
    };

    Ok(rows)
}

fn map_build_scan_row(row: SqliteRow) -> BuildScanRow {
    BuildScanRow {
        id: row.get("id"),
        build_tool_type: row.get("build_tool_type"),
        build_tool_version: row.get("build_tool_version"),
        plugin_version: row.get("plugin_version"),
        started_at: row.get("started_at"),
        finished_at: row.get("finished_at"),
        outcome: row.get("outcome"),
        requested_tasks: row.get("requested_tasks"),
        hostname: row.get("hostname"),
        os_name: row.get("os_name"),
        os_version: row.get("os_version"),
        jvm_vendor: row.get("jvm_vendor"),
        jvm_version: row.get("jvm_version"),
        created_at: row.get("created_at"),
    }
}

pub async fn get_build_scan(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<BuildScanRow>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, build_tool_type, build_tool_version, plugin_version, started_at, finished_at, outcome, requested_tasks, hostname, os_name, os_version, jvm_vendor, jvm_version, created_at \
         FROM build_scans WHERE id = ?",
    )
    .bind(id)
    .map(map_build_scan_row)
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
        sqlx::query(
            "SELECT id, scan_id, task_path, class_name, outcome, cacheable, start_timestamp, finish_timestamp, cache_key, origin_execution_time \
             FROM tasks WHERE scan_id = ? AND id > ? ORDER BY id LIMIT ?",
        )
        .bind(scan_id)
        .bind(cursor)
        .bind(limit)
        .map(map_task_row)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT id, scan_id, task_path, class_name, outcome, cacheable, start_timestamp, finish_timestamp, cache_key, origin_execution_time \
             FROM tasks WHERE scan_id = ? ORDER BY id LIMIT ?",
        )
        .bind(scan_id)
        .bind(limit)
        .map(map_task_row)
        .fetch_all(pool)
        .await?
    };

    Ok(rows)
}

fn map_task_row(row: SqliteRow) -> TaskRow {
    let cacheable_int: Option<i64> = row.get("cacheable");
    TaskRow {
        id: row.get("id"),
        scan_id: row.get("scan_id"),
        task_path: row.get("task_path"),
        class_name: row.get("class_name"),
        outcome: row.get("outcome"),
        cacheable: cacheable_int.map(|v| v != 0),
        start_timestamp: row.get("start_timestamp"),
        finish_timestamp: row.get("finish_timestamp"),
        cache_key: row.get("cache_key"),
        origin_execution_time: row.get("origin_execution_time"),
    }
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
    let row = sqlx::query(
        "SELECT id, scan_id, task_path, class_name, outcome, cacheable, start_timestamp, finish_timestamp, cache_key, origin_execution_time \
         FROM tasks WHERE id = ?",
    )
    .bind(id)
    .map(map_task_row)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}
