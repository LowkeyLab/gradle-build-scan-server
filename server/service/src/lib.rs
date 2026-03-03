use models::TaskOutcome;
use sqlx::SqlitePool;
use tracing::{error, info};

use domain;

pub struct UploadRequest {
    pub scan_id: String,
    pub build_tool_type: Option<String>,
    pub build_tool_version: Option<String>,
    pub plugin_version: Option<String>,
    pub raw_payload: Vec<u8>,
}

pub struct BuildScanService {
    pool: SqlitePool,
}

impl BuildScanService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn process_upload(&self, req: UploadRequest) {
        let scan_id = req.scan_id.clone();
        let payload_size = req.raw_payload.len();
        info!(scan_id = %scan_id, payload_size = payload_size, "Received build scan upload");

        match lib::parse(&req.raw_payload) {
            Err(e) => {
                error!(scan_id = %scan_id, error = %e, "Failed to parse build scan payload");
                if let Err(db_err) = db::insert_build_scan(
                    &self.pool,
                    &scan_id,
                    req.build_tool_type.as_deref().unwrap_or(""),
                    req.build_tool_version.as_deref().unwrap_or(""),
                    req.plugin_version.as_deref().unwrap_or(""),
                    None,
                    None,
                    Some("parse_error"),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    Some(&req.raw_payload),
                )
                .await
                {
                    error!(scan_id = %scan_id, error = %db_err, "Failed to store parse_error scan");
                }
            }
            Ok(payload) => {
                let outcome = if payload
                    .tasks
                    .iter()
                    .any(|t| matches!(t.outcome, Some(TaskOutcome::Failed)))
                {
                    "failed"
                } else {
                    "success"
                };

                let requested_tasks_json = if payload.requested_tasks.is_empty() {
                    None
                } else {
                    serde_json::to_string(&payload.requested_tasks).ok()
                };

                let mut tx = match self.pool.begin().await {
                    Ok(tx) => tx,
                    Err(e) => {
                        error!(scan_id = %scan_id, error = %e, "Failed to begin transaction");
                        return;
                    }
                };

                if let Err(db_err) = db::insert_build_scan(
                    &mut *tx,
                    &scan_id,
                    req.build_tool_type.as_deref().unwrap_or(""),
                    req.build_tool_version.as_deref().unwrap_or(""),
                    req.plugin_version.as_deref().unwrap_or(""),
                    None,
                    None,
                    Some(outcome),
                    requested_tasks_json.as_deref(),
                    payload.hostname.as_deref(),
                    payload.os_name.as_deref(),
                    payload.os_version.as_deref(),
                    payload.jvm_vendor.as_deref(),
                    payload.jvm_version.as_deref(),
                    Some(&req.raw_payload),
                )
                .await
                {
                    error!(scan_id = %scan_id, error = %db_err, "Failed to store build scan");
                    return;
                }

                let task_count = payload.tasks.len();
                for task in &payload.tasks {
                    let outcome_str = task.outcome.as_ref().map(|o| format!("{:?}", o));
                    let cache_key_str =
                        task.origin_build_cache_key.as_ref().map(|k| hex::encode(k));

                    if let Err(db_err) = db::insert_task(
                        &mut *tx,
                        &scan_id,
                        &task.task_path,
                        task.class_name.as_deref(),
                        outcome_str.as_deref(),
                        task.cacheable,
                        task.started_at,
                        task.finished_at,
                        cache_key_str.as_deref(),
                        None,
                    )
                    .await
                    {
                        error!(scan_id = %scan_id, task_path = %task.task_path, error = %db_err, "Failed to store task");
                        return;
                    }
                }

                if let Err(e) = tx.commit().await {
                    error!(scan_id = %scan_id, error = %e, "Failed to commit transaction");
                    return;
                }

                info!(scan_id = %scan_id, task_count = task_count, "Stored build scan successfully");
            }
        }
    }

    pub async fn get_build_scan(
        &self,
        id: &str,
    ) -> Result<Option<domain::BuildScan>, Box<dyn std::error::Error>> {
        let row = db::get_build_scan(&self.pool, id).await?;
        row.map(domain::BuildScan::try_from)
            .transpose()
            .map_err(|e| e.into())
    }

    pub async fn list_build_scans(
        &self,
        limit: i64,
        after_created_at: Option<&str>,
        after_id: Option<&str>,
    ) -> Result<Vec<domain::BuildScan>, Box<dyn std::error::Error>> {
        let rows = db::list_build_scans(&self.pool, limit, after_created_at, after_id).await?;
        rows.into_iter()
            .map(domain::BuildScan::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.into())
    }

    pub async fn get_task(
        &self,
        id: &str,
    ) -> Result<Option<domain::Task>, Box<dyn std::error::Error>> {
        let row = db::get_task(&self.pool, id).await?;
        row.map(domain::Task::try_from)
            .transpose()
            .map_err(|e| e.into())
    }

    pub async fn list_tasks(
        &self,
        scan_id: &str,
        limit: i64,
        after_id: Option<&str>,
    ) -> Result<Vec<domain::Task>, Box<dyn std::error::Error>> {
        let rows = db::list_tasks(&self.pool, scan_id, limit, after_id).await?;
        rows.into_iter()
            .map(domain::Task::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.into())
    }

    pub async fn count_tasks(
        &self,
        scan_id: &str,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        Ok(db::count_tasks(&self.pool, scan_id).await?)
    }

    pub async fn count_build_scans(&self) -> Result<i64, Box<dyn std::error::Error>> {
        Ok(db::count_build_scans(&self.pool).await?)
    }
}
