use anyhow::{Context, Result};
use chrono::Utc;
use models::TaskOutcome;
use sqlx::SqlitePool;
use tracing::{error, info};
use uuid::Uuid;

use domain;

fn map_cache_operation_type(op_type: &models::CacheOperationType) -> domain::CacheOperationType {
    match op_type {
        models::CacheOperationType::LocalLoad => domain::CacheOperationType::LocalLoad,
        models::CacheOperationType::RemoteLoad => domain::CacheOperationType::RemoteLoad,
        models::CacheOperationType::Pack => domain::CacheOperationType::Pack,
        models::CacheOperationType::Unpack => domain::CacheOperationType::Unpack,
        models::CacheOperationType::LocalStore => domain::CacheOperationType::LocalStore,
        models::CacheOperationType::RemoteStore => domain::CacheOperationType::RemoteStore,
    }
}

fn map_test_outcome(outcome: &models::TestOutcome) -> domain::TestOutcome {
    match outcome {
        models::TestOutcome::Passed => domain::TestOutcome::Passed,
        models::TestOutcome::Failed => domain::TestOutcome::Failed,
        models::TestOutcome::Skipped => domain::TestOutcome::Skipped,
    }
}

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

fn push_unique_string(values: &mut Vec<String>, value: String) {
    if !value.is_empty() && !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn configuration_labels(resolution: &models::ConfigurationResolutionData) -> Vec<String> {
    let mut labels = Vec::new();

    for label in &resolution.finished_labels {
        push_unique_string(&mut labels, label.clone());
    }
    for label in &resolution.started_labels {
        push_unique_string(&mut labels, label.clone());
    }

    labels
}

fn build_persisted_configuration_dependency(
    scan_id: domain::BuildScanId,
    resolution: &models::ConfigurationResolutionData,
) -> domain::PersistedConfigurationDependency {
    let labels = configuration_labels(resolution);
    let display_name = labels
        .first()
        .cloned()
        .unwrap_or_else(|| format!("Resolution {}", resolution.id));
    let details = labels
        .into_iter()
        .filter(|label| label != &display_name)
        .collect();

    domain::PersistedConfigurationDependency {
        scan_id,
        configuration_id: resolution.id,
        display_name,
        details,
        started_labels: resolution.started_labels.clone(),
        finished_labels: resolution.finished_labels.clone(),
    }
}

fn build_configuration_dependency_graph(
    configuration: &domain::ConfigurationDependency,
    dependency_labels: Vec<String>,
) -> Option<domain::ConfigurationDependencyGraph> {
    if dependency_labels.is_empty() {
        return None;
    }

    let root_id = format!("configuration:{}", configuration.id);
    let mut nodes = vec![domain::ConfigurationDependencyNode {
        id: root_id.clone(),
        label: configuration.display_name.clone(),
    }];
    let mut edges = Vec::new();

    for (index, label) in dependency_labels.into_iter().take(48).enumerate() {
        let node_id = format!("dependency:{}:{}", configuration.id, index);
        nodes.push(domain::ConfigurationDependencyNode {
            id: node_id.clone(),
            label,
        });
        edges.push(domain::ConfigurationDependencyEdge {
            source_id: root_id.clone(),
            target_id: node_id,
        });
    }

    Some(domain::ConfigurationDependencyGraph { nodes, edges })
}

fn map_configuration_dependency(
    dependency: domain::PersistedConfigurationDependency,
) -> domain::ConfigurationDependency {
    domain::ConfigurationDependency {
        id: dependency.configuration_id.to_string(),
        display_name: dependency.display_name,
        details: dependency.details,
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
                db::insert_build_scan(&self.pool, &scan, Some(&req.raw_payload))
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
                let persisted_scan_id = domain::BuildScanId(scan_uuid);

                let mut tx = self
                    .pool
                    .begin()
                    .await
                    .context("failed to begin transaction")?;

                db::insert_build_scan(&mut *tx, &scan, Some(&req.raw_payload))
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
                        task_builder.start_timestamp(ts.as_i64());
                    }
                    if let Some(ts) = task.finished_at {
                        task_builder.finish_timestamp(ts.as_i64());
                    }
                    if let Some(t) = task.origin_execution_time {
                        task_builder.origin_execution_time(t.as_i64());
                    }
                    if let Some(ck) = cache_key_str {
                        task_builder.cache_key(ck);
                    }
                    if let Some(ref r) = task.caching_disabled_reason {
                        task_builder.caching_disabled_reason(r.clone());
                    }
                    if let Some(ref e) = task.caching_disabled_explanation {
                        task_builder.caching_disabled_explanation(e.clone());
                    }
                    if let Some(ref msgs) = task.up_to_date_messages {
                        if !msgs.is_empty() {
                            task_builder.up_to_date_messages(msgs.clone());
                        }
                    }
                    if let Some(ref id) = task.origin_build_invocation_id {
                        task_builder.origin_build_invocation_id(id.clone());
                    }

                    let domain_task = task_builder
                        .build()
                        .map_err(|e| anyhow::anyhow!(e))
                        .context("failed to build Task")?;

                    db::insert_task(&mut *tx, &domain_task)
                        .await
                        .with_context(|| format!("failed to store task {}", task.task_path))?;

                    for cache_op in &task.cache_operations {
                        let succeeded = match cache_op.operation_type {
                            models::CacheOperationType::LocalLoad
                            | models::CacheOperationType::RemoteLoad => {
                                cache_op.hit.unwrap_or(false)
                            }
                            models::CacheOperationType::LocalStore
                            | models::CacheOperationType::RemoteStore => {
                                cache_op.stored.unwrap_or(false)
                            }
                            models::CacheOperationType::Pack
                            | models::CacheOperationType::Unpack => cache_op.failure_id.is_none(),
                        };
                        let domain_op = domain::CacheOperation {
                            id: domain::CacheOperationId(Uuid::new_v4()),
                            task_id: domain_task.id.clone(),
                            operation_type: map_cache_operation_type(&cache_op.operation_type),
                            succeeded,
                            archive_size: cache_op
                                .archive_size
                                .map(|s| domain::ArchiveSize(s.as_i64())),
                            cache_key: cache_op.cache_key.clone(),
                            duration_ms: cache_op.duration_ms.map(|d| domain::Duration(d.as_i64())),
                        };
                        db::insert_cache_operation(&mut *tx, &domain_op)
                            .await
                            .with_context(|| {
                                format!(
                                    "failed to store cache operation for task {}",
                                    task.task_path
                                )
                            })?;
                    }
                }

                let test_count = payload.tests.len();
                for test in &payload.tests {
                    let mut test_builder = domain::TestBuilder::default();
                    test_builder
                        .id(Uuid::new_v4())
                        .scan_id(scan_uuid)
                        .class_name(test.class_name.clone());

                    if let Some(mn) = &test.method_name {
                        test_builder.method_name(mn.clone());
                    }
                    if let Some(en) = &test.executor_name {
                        test_builder.executor_name(en.clone());
                    }
                    if let Some(o) = &test.outcome {
                        test_builder.outcome(map_test_outcome(o));
                    }
                    if let Some(d) = test.duration_ms {
                        test_builder.duration_ms(d.as_i64());
                    }

                    let domain_test = test_builder
                        .build()
                        .map_err(|e| anyhow::anyhow!(e))
                        .context("failed to build Test")?;

                    db::insert_test(&mut *tx, &domain_test)
                        .await
                        .with_context(|| format!("failed to store test {}", test.class_name))?;
                }

                let configuration_dependency_count = payload.configuration_resolutions.len();
                for resolution in &payload.configuration_resolutions {
                    let dependency = build_persisted_configuration_dependency(
                        persisted_scan_id.clone(),
                        resolution,
                    );

                    db::insert_configuration_dependency(&mut *tx, &dependency)
                        .await
                        .with_context(|| {
                            format!("failed to store configuration dependency {}", resolution.id)
                        })?;
                }

                let artifact_label_count =
                    if let Some(dependencies) = &payload.build_wide_configuration_dependencies {
                        for (ordinal, label) in dependencies.artifact_labels.iter().enumerate() {
                            let artifact_label =
                                domain::PersistedConfigurationDependencyArtifactLabel {
                                    scan_id: persisted_scan_id.clone(),
                                    ordinal: ordinal as i64,
                                    label: label.clone(),
                                };

                            db::insert_configuration_dependency_artifact_label(
                                &mut *tx,
                                &artifact_label,
                            )
                            .await
                            .with_context(|| {
                                format!(
                                    "failed to store configuration dependency artifact label {}",
                                    ordinal
                                )
                            })?;
                        }

                        dependencies.artifact_labels.len()
                    } else {
                        0
                    };

                tx.commit().await.context("failed to commit transaction")?;

                info!(scan_id = %scan_id, task_count = task_count, test_count = test_count, configuration_dependency_count = configuration_dependency_count, artifact_label_count = artifact_label_count, "Stored build scan successfully");
            }
        }

        Ok(())
    }

    pub async fn get_build_scan(&self, id: &str) -> Result<Option<domain::BuildScan>> {
        db::get_build_scan(&self.pool, id).await
    }

    pub async fn get_configuration_dependencies(
        &self,
        scan_id: &str,
    ) -> Result<Vec<domain::ConfigurationDependency>> {
        Ok(db::list_configuration_dependencies(&self.pool, scan_id)
            .await?
            .into_iter()
            .map(map_configuration_dependency)
            .collect())
    }

    pub async fn get_configuration_dependency_graph(
        &self,
        scan_id: &str,
        configuration_id: &str,
    ) -> Result<Option<domain::ConfigurationDependencyGraph>> {
        let Ok(configuration_id) = configuration_id.parse::<i64>() else {
            return Ok(None);
        };

        let Some(configuration) =
            db::get_configuration_dependency(&self.pool, scan_id, configuration_id).await?
        else {
            return Ok(None);
        };

        let artifact_labels =
            db::list_configuration_dependency_artifact_labels(&self.pool, scan_id)
                .await?
                .into_iter()
                .map(|label| label.label)
                .collect::<Vec<_>>();
        let configuration = map_configuration_dependency(configuration.clone());
        let dependency_labels = if artifact_labels.is_empty() {
            configuration.details.clone()
        } else {
            artifact_labels
        };

        Ok(build_configuration_dependency_graph(
            &configuration,
            dependency_labels,
        ))
    }

    pub async fn list_build_scans(
        &self,
        limit: i64,
        after_created_at: Option<&str>,
        after_id: Option<&str>,
    ) -> Result<Vec<domain::BuildScan>> {
        db::list_build_scans(&self.pool, limit, after_created_at, after_id).await
    }

    pub async fn get_task(&self, id: &str) -> Result<Option<domain::Task>> {
        db::get_task(&self.pool, id).await
    }

    pub async fn list_tasks(
        &self,
        scan_id: &str,
        limit: i64,
        after_id: Option<&str>,
    ) -> Result<Vec<domain::Task>> {
        db::list_tasks(&self.pool, scan_id, limit, after_id).await
    }

    pub async fn count_tasks(&self, scan_id: &str) -> Result<i64> {
        db::count_tasks(&self.pool, scan_id).await
    }

    pub async fn count_build_scans(&self) -> Result<i64> {
        db::count_build_scans(&self.pool).await
    }

    pub async fn list_tests(
        &self,
        scan_id: &str,
        limit: i64,
        after_id: Option<&str>,
    ) -> Result<Vec<domain::Test>> {
        db::list_tests(&self.pool, scan_id, limit, after_id).await
    }

    pub async fn count_tests(&self, scan_id: &str) -> Result<i64> {
        db::count_tests(&self.pool, scan_id).await
    }

    pub async fn test_summary(&self, scan_id: &str) -> Result<domain::TestSummary> {
        db::test_summary(&self.pool, scan_id).await
    }

    pub async fn get_test(&self, id: &str) -> Result<Option<domain::Test>> {
        db::get_test(&self.pool, id).await
    }

    pub async fn list_cache_operations(
        &self,
        task_id: &str,
    ) -> Result<Vec<domain::CacheOperation>> {
        db::list_cache_operations(&self.pool, task_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn persisted_configuration_dependency(
        configuration_id: i64,
        display_name: &str,
        details: &[&str],
    ) -> domain::PersistedConfigurationDependency {
        domain::PersistedConfigurationDependency {
            scan_id: domain::BuildScanId(Uuid::nil()),
            configuration_id,
            display_name: display_name.to_string(),
            details: details.iter().map(|detail| detail.to_string()).collect(),
            started_labels: Vec::new(),
            finished_labels: Vec::new(),
        }
    }

    #[test]
    fn configuration_dependency_graph_uses_persisted_artifact_labels() {
        let configuration = map_configuration_dependency(persisted_configuration_dependency(
            6648731359144961106,
            ":app:build",
            &[":list:build"],
        ));

        let graph = build_configuration_dependency_graph(
            &configuration,
            vec![
                "junit-jupiter-api-5.12.1.jar".to_string(),
                "hamcrest-3.0.jar".to_string(),
            ],
        )
        .expect("graph should be built");

        assert_eq!(graph.nodes[0].id, "configuration:6648731359144961106");
        assert_eq!(graph.nodes[0].label, ":app:build");
        assert_eq!(graph.nodes[1].label, "junit-jupiter-api-5.12.1.jar");
        assert_eq!(graph.nodes[2].label, "hamcrest-3.0.jar");
        assert_eq!(
            graph.edges[0].source_id,
            "configuration:6648731359144961106"
        );
        assert_eq!(graph.edges[0].target_id, "dependency:6648731359144961106:0");
    }

    #[test]
    fn persisted_configuration_dependency_keeps_raw_labels_and_derives_summary() {
        let dependency = build_persisted_configuration_dependency(
            domain::BuildScanId(Uuid::nil()),
            &models::ConfigurationResolutionData {
                id: 19,
                started_labels: vec![
                    ":app:compileClasspath".to_string(),
                    "shared-label".to_string(),
                ],
                finished_labels: vec![
                    ":app:runtimeClasspath".to_string(),
                    "shared-label".to_string(),
                ],
            },
        );

        assert_eq!(dependency.configuration_id, 19);
        assert_eq!(dependency.display_name, ":app:runtimeClasspath");
        assert_eq!(
            dependency.details,
            vec![
                "shared-label".to_string(),
                ":app:compileClasspath".to_string()
            ]
        );
        assert_eq!(
            dependency.started_labels,
            vec![
                ":app:compileClasspath".to_string(),
                "shared-label".to_string(),
            ]
        );
        assert_eq!(
            dependency.finished_labels,
            vec![
                ":app:runtimeClasspath".to_string(),
                "shared-label".to_string(),
            ]
        );
    }

    #[test]
    fn configuration_dependency_graph_falls_back_to_persisted_details() {
        let configuration = map_configuration_dependency(persisted_configuration_dependency(
            77,
            ":app:build",
            &[":list:build", ":utilities:build"],
        ));

        let graph =
            build_configuration_dependency_graph(&configuration, configuration.details.clone())
                .expect("graph should be built");

        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.nodes[1].label, ":list:build");
        assert_eq!(graph.nodes[2].label, ":utilities:build");
    }

    #[test]
    fn configuration_dependency_graph_is_absent_without_any_persisted_labels() {
        let configuration =
            map_configuration_dependency(persisted_configuration_dependency(77, ":app:build", &[]));

        assert!(build_configuration_dependency_graph(&configuration, Vec::new()).is_none());
    }
}
