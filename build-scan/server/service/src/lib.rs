use anyhow::{Context, Result};
use chrono::Utc;
use models::TaskOutcome;
use sqlx::SqlitePool;
use tracing::{error, info};
use uuid::Uuid;

use domain;

pub struct UploadRequest {
    pub scan_id: String,
    pub build_tool_type: Option<String>,
    pub build_tool_version: Option<String>,
    pub plugin_version: Option<String>,
    pub raw_payload: Vec<u8>,
}

fn map_task_outcome(outcome: &models::TaskOutcome) -> domain::TaskOutcome {
    match outcome {
        models::TaskOutcome::UpToDate => domain::TaskOutcome::UpToDate,
        models::TaskOutcome::Skipped => domain::TaskOutcome::Skipped,
        models::TaskOutcome::Failed => domain::TaskOutcome::Failed,
        models::TaskOutcome::Success => domain::TaskOutcome::Success,
        models::TaskOutcome::FromCache => domain::TaskOutcome::FromCache,
        models::TaskOutcome::NoSource => domain::TaskOutcome::NoSource,
        models::TaskOutcome::AvoidedForUnknownReason => {
            domain::TaskOutcome::AvoidedForUnknownReason
        }
    }
}

pub struct BuildScanService {
    pool: SqlitePool,
}

impl BuildScanService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn process_upload(&self, req: UploadRequest) -> Result<()> {
        let scan_id = req.scan_id.clone();
        let scan_uuid = Uuid::parse_str(&scan_id).context("invalid scan_id UUID")?;
        let payload_size = req.raw_payload.len();
        info!(scan_id = %scan_id, payload_size = payload_size, "Received build scan upload");

        match lib::parse(&req.raw_payload) {
            Err(e) => {
                error!(scan_id = %scan_id, error = %e, "Failed to parse build scan payload");
                let scan = domain::BuildScanBuilder::default()
                    .id(scan_uuid)
                    .build_tool_type(req.build_tool_type.unwrap_or_default())
                    .build_tool_version(req.build_tool_version.unwrap_or_default())
                    .plugin_version(req.plugin_version.unwrap_or_default())
                    .outcome(domain::BuildOutcome::ParseError)
                    .created_at(Utc::now())
                    .build()
                    .map_err(|e| anyhow::anyhow!(e))
                    .context("failed to build parse-error BuildScan")?;
                let row = db::BuildScanRow::from(&scan);
                db::insert_build_scan(&self.pool, &row, Some(&req.raw_payload))
                    .await
                    .context("failed to store parse_error scan")?;
            }
            Ok(payload) => {
                let outcome = if payload
                    .tasks
                    .iter()
                    .any(|t| matches!(t.outcome, Some(TaskOutcome::Failed)))
                {
                    domain::BuildOutcome::Failed
                } else {
                    domain::BuildOutcome::Success
                };

                let requested_tasks: Vec<domain::RequestedTask> = payload
                    .requested_tasks
                    .into_iter()
                    .map(domain::RequestedTask::from)
                    .collect();

                let mut scan_builder = domain::BuildScanBuilder::default();
                scan_builder
                    .id(scan_uuid)
                    .build_tool_type(req.build_tool_type.unwrap_or_default())
                    .build_tool_version(req.build_tool_version.unwrap_or_default())
                    .plugin_version(req.plugin_version.unwrap_or_default())
                    .outcome(outcome)
                    .created_at(Utc::now());

                if !requested_tasks.is_empty() {
                    scan_builder.requested_tasks(requested_tasks);
                }
                if let Some(h) = payload.hostname {
                    scan_builder.hostname(h);
                }
                if let Some(n) = payload.os_name {
                    scan_builder.os_name(n);
                }
                if let Some(v) = payload.os_version {
                    scan_builder.os_version(v);
                }
                if let Some(v) = payload.jvm_vendor {
                    scan_builder.jvm_vendor(v);
                }
                if let Some(v) = payload.jvm_version {
                    scan_builder.jvm_version(v);
                }

                let scan = scan_builder
                    .build()
                    .map_err(|e| anyhow::anyhow!(e))
                    .context("failed to build BuildScan")?;

                let mut tx = self
                    .pool
                    .begin()
                    .await
                    .context("failed to begin transaction")?;

                let row = db::BuildScanRow::from(&scan);
                db::insert_build_scan(&mut *tx, &row, Some(&req.raw_payload))
                    .await
                    .context("failed to store build scan")?;

                let task_count = payload.tasks.len();
                for task in &payload.tasks {
                    let task_outcome = task.outcome.as_ref().map(map_task_outcome);

                    let cache_key_str =
                        task.origin_build_cache_key.as_ref().map(|k| hex::encode(k));

                    let mut task_builder = domain::TaskBuilder::default();
                    task_builder
                        .id(Uuid::new_v4())
                        .scan_id(scan_uuid)
                        .task_path(task.task_path.clone());

                    if let Some(cn) = &task.class_name {
                        task_builder.class_name(cn.clone());
                    }
                    if let Some(o) = task_outcome {
                        task_builder.outcome(o);
                    }
                    if let Some(c) = task.cacheable {
                        task_builder.cacheable(c);
                    }
                    if let Some(ts) = task.started_at {
                        task_builder.start_timestamp(ts);
                    }
                    if let Some(ts) = task.finished_at {
                        task_builder.finish_timestamp(ts);
                    }
                    if let Some(d) = task.duration_ms {
                        task_builder.origin_execution_time(d);
                    }
                    if let Some(ck) = cache_key_str {
                        task_builder.cache_key(ck);
                    }

                    let domain_task = task_builder
                        .build()
                        .map_err(|e| anyhow::anyhow!(e))
                        .context("failed to build Task")?;

                    let task_row = db::TaskRow::from(&domain_task);
                    db::insert_task(&mut *tx, &task_row)
                        .await
                        .with_context(|| format!("failed to store task {}", task.task_path))?;
                }

                tx.commit().await.context("failed to commit transaction")?;

                info!(scan_id = %scan_id, task_count = task_count, "Stored build scan successfully");
            }
        }

        Ok(())
    }

    pub async fn get_build_scan(&self, id: &str) -> Result<Option<domain::BuildScan>> {
        let row = db::get_build_scan(&self.pool, id).await?;
        Ok(row.map(domain::BuildScan::try_from).transpose()?)
    }

    pub async fn list_build_scans(
        &self,
        limit: i64,
        after_created_at: Option<&str>,
        after_id: Option<&str>,
    ) -> Result<Vec<domain::BuildScan>> {
        let rows = db::list_build_scans(&self.pool, limit, after_created_at, after_id).await?;
        Ok(rows
            .into_iter()
            .map(domain::BuildScan::try_from)
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub async fn get_task(&self, id: &str) -> Result<Option<domain::Task>> {
        let row = db::get_task(&self.pool, id).await?;
        Ok(row.map(domain::Task::try_from).transpose()?)
    }

    pub async fn list_tasks(
        &self,
        scan_id: &str,
        limit: i64,
        after_id: Option<&str>,
    ) -> Result<Vec<domain::Task>> {
        let rows = db::list_tasks(&self.pool, scan_id, limit, after_id).await?;
        Ok(rows
            .into_iter()
            .map(domain::Task::try_from)
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub async fn count_tasks(&self, scan_id: &str) -> Result<i64> {
        Ok(db::count_tasks(&self.pool, scan_id).await?)
    }

    pub async fn count_build_scans(&self) -> Result<i64> {
        Ok(db::count_build_scans(&self.pool).await?)
    }
}
