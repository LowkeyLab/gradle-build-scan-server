use std::collections::HashMap;

use events::DecodedEvent;
use framing::FramedEvent;
use models::{BuildScanPayload, RawEventSummary, Task, TaskId, TaskOutcome};

pub fn assemble(events: Vec<(FramedEvent, DecodedEvent)>) -> BuildScanPayload {
    let mut identities: HashMap<TaskId, (String, String)> = HashMap::new();
    let mut started: HashMap<TaskId, (String, Option<String>, i64)> = HashMap::new();
    let mut finished: HashMap<TaskId, FinishedInfo> = HashMap::new();
    let mut raw_counts: HashMap<u16, usize> = HashMap::new();
    let mut property_names_map: HashMap<TaskId, events::TaskInputsPropertyNamesEvent> =
        HashMap::new();
    let mut implementation_map: HashMap<TaskId, events::TaskInputsImplementationEvent> =
        HashMap::new();
    let mut value_properties_map: HashMap<TaskId, events::TaskInputsValuePropertiesEvent> =
        HashMap::new();
    let mut file_property_roots_map: HashMap<TaskId, Vec<events::TaskInputsFilePropertyRootEvent>> =
        HashMap::new();
    let mut file_properties_map: HashMap<TaskId, Vec<events::TaskInputsFilePropertyEvent>> =
        HashMap::new();
    let mut snapshotting_finished_map: HashMap<
        TaskId,
        events::TaskInputsSnapshottingFinishedEvent,
    > = HashMap::new();
    let mut planned_nodes: Vec<events::PlannedNodeEvent> = Vec::new();
    let mut transform_requests: Vec<events::TransformExecutionRequestEvent> = Vec::new();
    // Test events are strictly interleaved: each TestCase is followed by its TestResult.
    // We collect all TestCase events (including non-method metadata) and all TestResult events
    // in order, then pair them positionally to assign outcomes.
    let mut all_test_cases: Vec<Option<models::TestCase>> = Vec::new();
    let mut test_results: Vec<models::TestOutcome> = Vec::new();
    let mut executor_names: HashMap<models::ExecutorId, String> = HashMap::new();
    let mut task_registration_summary: Option<events::TaskRegistrationSummaryEvent> = None;
    let mut basic_memory_stats: Option<events::BasicMemoryStatsEvent> = None;
    let mut resource_usage: Option<events::ResourceUsageEvent> = None;
    let mut build_agent: Option<events::BuildAgentEvent> = None;
    let mut os_event: Option<events::OsEvent> = None;
    let mut jvm_event: Option<events::JvmEvent> = None;
    let mut requested_tasks: Vec<String> = Vec::new();

    for (frame, decoded) in &events {
        match decoded {
            DecodedEvent::TaskIdentity(e) => {
                identities.insert(e.id, (e.build_path.clone(), e.task_path.clone()));
            }
            DecodedEvent::TaskStarted(e) => {
                started.insert(
                    e.id,
                    (e.build_path.clone(), e.class_name.clone(), frame.timestamp),
                );
            }
            DecodedEvent::TaskFinished(e) => {
                finished.insert(
                    e.id,
                    FinishedInfo {
                        outcome: e.outcome.and_then(TaskOutcome::from_ordinal),
                        cacheable: e.cacheable,
                        caching_disabled_reason: e.caching_disabled_reason_category.clone(),
                        caching_disabled_explanation: e.caching_disabled_explanation.clone(),
                        origin_build_cache_key: e.origin_build_cache_key.clone(),
                        origin_execution_time: e.origin_execution_time,
                        actionable: e.actionable,
                        timestamp: frame.timestamp,
                        up_to_date_messages: e.up_to_date_messages.clone(),
                        origin_build_invocation_id: e.origin_build_invocation_id.clone(),
                    },
                );
            }
            DecodedEvent::TaskInputsPropertyNames(e) => {
                if let Some(id) = e.id {
                    property_names_map.insert(id, e.clone());
                }
            }
            DecodedEvent::TaskInputsImplementation(e) => {
                if let Some(id) = e.id {
                    implementation_map.insert(id, e.clone());
                }
            }
            DecodedEvent::TaskInputsValueProperties(e) => {
                if let Some(id) = e.id {
                    value_properties_map.insert(id, e.clone());
                }
            }
            DecodedEvent::TaskInputsFilePropertyRoot(e) => {
                if let Some(id) = e.id {
                    file_property_roots_map
                        .entry(id)
                        .or_default()
                        .push(e.clone());
                }
            }
            DecodedEvent::TaskInputsFileProperty(e) => {
                if let Some(id) = e.id {
                    file_properties_map.entry(id).or_default().push(e.clone());
                }
            }
            DecodedEvent::TaskInputsSnapshottingStarted(_) => {}
            DecodedEvent::TaskInputsSnapshottingFinished(e) => {
                if let Some(task_id) = e.task {
                    snapshotting_finished_map.insert(task_id, e.clone());
                }
            }
            DecodedEvent::PlannedNode(e) => {
                planned_nodes.push(e.clone());
            }
            DecodedEvent::TransformExecutionRequest(e) => {
                transform_requests.push(e.clone());
            }
            DecodedEvent::TaskRegistrationSummary(e) => {
                if task_registration_summary.is_none() {
                    task_registration_summary = Some(e.clone());
                }
            }
            DecodedEvent::BasicMemoryStats(e) => {
                if basic_memory_stats.is_none() {
                    basic_memory_stats = Some(e.clone());
                }
            }
            DecodedEvent::ResourceUsage(e) => {
                if resource_usage.is_none() {
                    resource_usage = Some(e.clone());
                }
            }
            // Decoded for protocol coverage; not yet consumed by assembly.
            DecodedEvent::JavaToolchainUsage(_) => {}
            DecodedEvent::TransformExecutionStarted(_) => {}
            DecodedEvent::TransformIdentification(_) => {}
            DecodedEvent::TransformExecutionFinished(_) => {}
            DecodedEvent::OutputStyledText(_) => {}
            DecodedEvent::BuildStarted => {}
            DecodedEvent::BuildAgent(e) => {
                if build_agent.is_none() {
                    build_agent = Some(e.clone());
                }
            }
            DecodedEvent::BuildRequestedTasks(e) => {
                if requested_tasks.is_empty() {
                    requested_tasks = e.requested.clone();
                }
            }
            DecodedEvent::BuildFinished(_) => {}
            DecodedEvent::BuildModes(_) => {}
            DecodedEvent::DaemonState(_) => {}
            DecodedEvent::Encoding(_) => {}
            DecodedEvent::FileRefRoots(_) => {}
            DecodedEvent::Hardware(_) => {}
            DecodedEvent::Jvm(e) => {
                if jvm_event.is_none() {
                    jvm_event = Some(e.clone());
                }
            }
            DecodedEvent::JvmArgs(_) => {}
            DecodedEvent::Locality(_) => {}
            DecodedEvent::Os(e) => {
                if os_event.is_none() {
                    os_event = Some(e.clone());
                }
            }
            DecodedEvent::ScopeIds(_) => {}
            DecodedEvent::TestExecutorIdentity(e) => {
                executor_names.insert(e.executor_id, e.name.clone());
            }
            DecodedEvent::TestCase(e) => {
                // Track ALL TestCase events (including non-method metadata) to maintain
                // positional correspondence with TestResult events.
                if e.method_name.is_some() {
                    let executor_name = e
                        .executor_name
                        .clone()
                        .or_else(|| executor_names.get(&e.executor_id).cloned());
                    all_test_cases.push(Some(models::TestCase {
                        class_name: e.class_name.clone(),
                        method_name: e.method_name.clone(),
                        executor_name,
                        outcome: None,
                    }));
                } else {
                    // Non-method event (class-level or executor-init): placeholder to
                    // keep positional alignment with TestResult events.
                    all_test_cases.push(None);
                }
            }
            DecodedEvent::TestExecutorStarted(_) => {}
            DecodedEvent::TestExecutorFinished(_) => {}
            DecodedEvent::TestResult(e) => {
                let outcome = if e.failed {
                    models::TestOutcome::Failed
                } else if e.skipped {
                    models::TestOutcome::Skipped
                } else {
                    models::TestOutcome::Passed
                };
                test_results.push(outcome);
            }
            DecodedEvent::BuildCacheLocalLoadStarted(_) => {}
            DecodedEvent::BuildCacheLocalLoadFinished(_) => {}
            DecodedEvent::BuildCacheRemoteLoadStarted(_) => {}
            DecodedEvent::BuildCacheRemoteLoadFinished(_) => {}
            DecodedEvent::BuildCachePackStarted(_) => {}
            DecodedEvent::BuildCachePackFinished(_) => {}
            DecodedEvent::BuildCacheUnpackStarted(_) => {}
            DecodedEvent::BuildCacheUnpackFinished(_) => {}
            DecodedEvent::BuildCacheLocalStoreStarted(_) => {}
            DecodedEvent::BuildCacheLocalStoreFinished(_) => {}
            DecodedEvent::BuildCacheRemoteStoreStarted(_) => {}
            DecodedEvent::BuildCacheRemoteStoreFinished(_) => {}
            DecodedEvent::Raw(r) => {
                *raw_counts.entry(r.wire_id).or_insert(0) += 1;
            }
        }
    }

    let mut tasks: Vec<Task> = identities
        .into_iter()
        .map(|(id, (build_path, task_path))| {
            let (class_name, started_at) = started
                .get(&id)
                .map(|(_, cn, ts)| (cn.clone(), Some(*ts)))
                .unwrap_or((None, None));
            let fin = finished.get(&id);
            let finished_at = fin.map(|f| f.timestamp);
            let duration_ms = match (started_at, finished_at) {
                (Some(s), Some(f)) => Some(f - s),
                _ => None,
            };
            let inputs =
                {
                    let pn = property_names_map.remove(&id).map(|e| {
                        models::TaskInputsPropertyNamesData {
                            value_inputs: e.value_inputs,
                            file_inputs: e.file_inputs,
                            outputs: e.outputs,
                        }
                    });
                    let imp = implementation_map.remove(&id).map(|e| {
                        models::TaskInputsImplementationData {
                            class_loader_hash: e.class_loader_hash,
                            action_class_loader_hashes: e.action_class_loader_hashes,
                            action_class_names: e.action_class_names,
                        }
                    });
                    let vp = value_properties_map
                        .remove(&id)
                        .map(|e| models::TaskInputsValuePropertiesData { hashes: e.hashes });
                    let fpr = file_property_roots_map
                        .remove(&id)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|e| models::TaskInputsFilePropertyRootData {
                            file_root: e.file.root,
                            file_path: e.file.path,
                            root_hash: e.root_hash,
                            children: e
                                .children
                                .into_iter()
                                .map(|c| models::FilePropertyRootChildData {
                                    name: c.name,
                                    hash: c.hash,
                                    parent: c.parent,
                                })
                                .collect(),
                        })
                        .collect::<Vec<_>>();
                    let fp = file_properties_map
                        .remove(&id)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|e| models::TaskInputsFilePropertyData {
                            attributes: e.attributes,
                            hash: e.hash,
                            roots: e.roots,
                        })
                        .collect::<Vec<_>>();
                    let sr = snapshotting_finished_map.remove(&id).and_then(|e| {
                        e.result.map(|r| models::TaskInputsSnapshottingResultData {
                            hash: r.hash,
                            implementation: r.implementation,
                            property_names: r.property_names,
                            value_inputs: r.value_inputs,
                            file_inputs: r.file_inputs,
                        })
                    });
                    if pn.is_some()
                        || imp.is_some()
                        || vp.is_some()
                        || !fpr.is_empty()
                        || !fp.is_empty()
                        || sr.is_some()
                    {
                        Some(models::TaskInputs {
                            property_names: pn,
                            implementation: imp,
                            value_properties: vp,
                            file_property_roots: fpr,
                            file_properties: fp,
                            snapshotting_result: sr,
                        })
                    } else {
                        None
                    }
                };
            Task {
                id,
                build_path,
                task_path,
                class_name,
                outcome: fin.and_then(|f| f.outcome.clone()),
                cacheable: fin.and_then(|f| f.cacheable),
                caching_disabled_reason: fin.and_then(|f| f.caching_disabled_reason.clone()),
                caching_disabled_explanation: fin
                    .and_then(|f| f.caching_disabled_explanation.clone()),
                origin_build_cache_key: fin.and_then(|f| f.origin_build_cache_key.clone()),
                actionable: fin.and_then(|f| f.actionable),
                started_at,
                finished_at,
                duration_ms,
                inputs,
                up_to_date_messages: fin.and_then(|f| f.up_to_date_messages.clone()),
                origin_build_invocation_id: fin.and_then(|f| f.origin_build_invocation_id.clone()),
                origin_execution_time: fin.and_then(|f| f.origin_execution_time),
            }
        })
        .collect();

    tasks.sort_by_key(|t| t.id);

    let mut raw_events: Vec<RawEventSummary> = raw_counts
        .into_iter()
        .map(|(wire_id, count)| RawEventSummary { wire_id, count })
        .collect();
    raw_events.sort_by_key(|r| r.wire_id);

    let planned_nodes_data: Vec<models::PlannedNodeData> = planned_nodes
        .into_iter()
        .map(|e| models::PlannedNodeData {
            id: e.id,
            dependencies: e.dependencies,
            must_run_after: e.must_run_after,
            should_run_after: e.should_run_after,
            finalized_by: e.finalized_by,
        })
        .collect();

    let transform_requests_data: Vec<models::TransformExecutionRequestData> = transform_requests
        .into_iter()
        .map(|e| models::TransformExecutionRequestData {
            node_id: e.node_id,
            identification_id: e.identification_id,
            execution_id: e.execution_id,
        })
        .collect();

    BuildScanPayload {
        tasks,
        planned_nodes: planned_nodes_data,
        transform_execution_requests: transform_requests_data,
        raw_events,
        task_registration_summary: task_registration_summary.map(|e| {
            models::TaskRegistrationSummaryData {
                task_count: e.task_count,
            }
        }),
        basic_memory_stats: basic_memory_stats.map(|e| models::BasicMemoryStatsData {
            free: e.free,
            total: e.total,
            max: e.max,
            peak_snapshots: e
                .peak_snapshots
                .into_iter()
                .map(|s| models::MemoryPoolSnapshotData {
                    name: s.name,
                    heap: s.heap,
                    init: s.init,
                    used: s.used,
                    committed: s.committed,
                    max: s.max,
                })
                .collect(),
            gc_time: e.gc_time,
        }),
        hostname: build_agent.and_then(|a| a.local_hostname),
        os_name: os_event.as_ref().and_then(|o| o.name.clone()),
        os_version: os_event.and_then(|o| o.version),
        jvm_vendor: jvm_event.as_ref().and_then(|j| j.vendor.clone()),
        jvm_version: jvm_event.and_then(|j| j.version),
        requested_tasks,
        tests: {
            debug_assert_eq!(
                all_test_cases.len(),
                test_results.len(),
                "TestCase/TestResult count mismatch: {} cases vs {} results",
                all_test_cases.len(),
                test_results.len(),
            );
            all_test_cases
                .into_iter()
                .zip(
                    test_results
                        .into_iter()
                        .map(Some)
                        .chain(std::iter::repeat(None)),
                )
                .filter_map(|(tc, outcome)| {
                    tc.map(|mut test_case| {
                        test_case.outcome = outcome;
                        test_case
                    })
                })
                .collect()
        },
        resource_usage: resource_usage.map(|e| models::ResourceUsageData {
            timestamps: e.timestamps,
            build_process_cpu: assemble_normalized_samples(e.build_process_cpu),
            build_child_processes_cpu: assemble_normalized_samples(e.build_child_processes_cpu),
            all_processes_cpu_sum: assemble_normalized_samples(e.all_processes_cpu_sum),
            all_processes_cpu: e.all_processes_cpu,
            build_process_memory: assemble_normalized_samples(e.build_process_memory),
            build_child_processes_memory: assemble_normalized_samples(
                e.build_child_processes_memory,
            ),
            all_processes_memory: assemble_normalized_samples(e.all_processes_memory),
            total_system_memory: e.total_system_memory,
            disk_read_speed: assemble_normalized_samples(e.disk_read_speed),
            disk_write_speed: assemble_normalized_samples(e.disk_write_speed),
            network_download_speed: assemble_normalized_samples(e.network_download_speed),
            network_upload_speed: assemble_normalized_samples(e.network_upload_speed),
            processes: e
                .processes
                .into_iter()
                .map(|p| models::ProcessData {
                    id: p.id,
                    name: p.name,
                    display_name: p.display_name,
                    process_type: p.process_type.map(process_type_name),
                })
                .collect(),
            top_processes_by_cpu: assemble_indexed_normalized_samples(e.top_processes_by_cpu),
            top_processes_by_memory: assemble_indexed_normalized_samples(e.top_processes_by_memory),
        }),
    }
}

fn process_type_name(ordinal: u64) -> String {
    match ordinal {
        0 => "Self".to_string(),
        1 => "Descendant".to_string(),
        2 => "Other".to_string(),
        _ => format!("Unknown({})", ordinal),
    }
}

fn assemble_normalized_samples(e: events::NormalizedSamplesEvent) -> models::NormalizedSamplesData {
    models::NormalizedSamplesData {
        samples: e.samples,
        max: e.max,
    }
}

fn assemble_indexed_normalized_samples(
    e: events::IndexedNormalizedSamplesEvent,
) -> models::IndexedNormalizedSamplesData {
    models::IndexedNormalizedSamplesData {
        indices: e.indices,
        samples: e.samples,
        max: e.max,
    }
}

struct FinishedInfo {
    outcome: Option<TaskOutcome>,
    cacheable: Option<bool>,
    caching_disabled_reason: Option<String>,
    caching_disabled_explanation: Option<String>,
    origin_build_cache_key: Option<Vec<u8>>,
    origin_execution_time: Option<i64>,
    actionable: Option<bool>,
    timestamp: i64,
    up_to_date_messages: Option<Vec<String>>,
    origin_build_invocation_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use events::*;
    use models::{ExecutorId, TaskId};

    fn frame(wire_id: u16, ts: i64) -> FramedEvent {
        FramedEvent {
            wire_id,
            timestamp: ts,
            ordinal: 0,
            body: vec![],
        }
    }

    #[test]
    fn test_assemble_test_cases() {
        let events = vec![
            (
                frame(127, 1000),
                DecodedEvent::TestExecutorIdentity(TestExecutorIdentityEvent {
                    executor_id: ExecutorId::new(42),
                    name: "Gradle Test Executor 1".into(),
                }),
            ),
            (
                frame(798, 2000),
                DecodedEvent::TestCase(TestCaseEvent {
                    executor_id: ExecutorId::new(42),
                    class_name: "org.example.list.LinkedListTest".into(),
                    method_name: Some("testAdd()".into()),
                    executor_name: Some("Gradle Test Executor 1".into()),
                }),
            ),
            (
                frame(284, 2000),
                DecodedEvent::TestResult(TestResultEvent {
                    task: TaskId::new(1),
                    id: 100,
                    failed: false,
                    skipped: false,
                }),
            ),
            (
                frame(798, 2001),
                DecodedEvent::TestCase(TestCaseEvent {
                    executor_id: ExecutorId::new(42),
                    class_name: "org.example.list.LinkedListTest".into(),
                    method_name: None,
                    executor_name: None,
                }),
            ),
            (
                frame(284, 2001),
                DecodedEvent::TestResult(TestResultEvent {
                    task: TaskId::new(1),
                    id: 101,
                    failed: false,
                    skipped: false,
                }),
            ),
        ];
        let payload = assemble(events);
        // Only method-level events become test cases
        assert_eq!(payload.tests.len(), 1);
        assert_eq!(
            payload.tests[0].class_name,
            "org.example.list.LinkedListTest"
        );
        assert_eq!(payload.tests[0].method_name.as_deref(), Some("testAdd()"));
        assert_eq!(
            payload.tests[0].executor_name.as_deref(),
            Some("Gradle Test Executor 1")
        );
    }

    #[test]
    fn test_assemble_test_cases_with_outcome() {
        // Events are interleaved: TestCase followed by its TestResult
        let events = vec![
            (
                frame(798, 1000),
                DecodedEvent::TestCase(TestCaseEvent {
                    executor_id: ExecutorId::new(42),
                    class_name: "org.example.MyTest".into(),
                    method_name: Some("testPass()".into()),
                    executor_name: None,
                }),
            ),
            (
                frame(284, 1001),
                DecodedEvent::TestResult(TestResultEvent {
                    task: TaskId::new(1),
                    id: 100,
                    failed: false,
                    skipped: false,
                }),
            ),
            (
                frame(798, 2000),
                DecodedEvent::TestCase(TestCaseEvent {
                    executor_id: ExecutorId::new(99),
                    class_name: "org.example.MyTest".into(),
                    method_name: Some("testFail()".into()),
                    executor_name: None,
                }),
            ),
            (
                frame(284, 2001),
                DecodedEvent::TestResult(TestResultEvent {
                    task: TaskId::new(1),
                    id: 200,
                    failed: true,
                    skipped: false,
                }),
            ),
        ];
        let payload = assemble(events);
        assert_eq!(payload.tests.len(), 2);
        assert!(
            matches!(payload.tests[0].outcome, Some(models::TestOutcome::Passed)),
            "expected Passed, got {:?}",
            payload.tests[0].outcome
        );
        assert!(
            matches!(payload.tests[1].outcome, Some(models::TestOutcome::Failed)),
            "expected Failed, got {:?}",
            payload.tests[1].outcome
        );
    }

    #[test]
    fn test_assemble_single_task() {
        let events = vec![
            (
                frame(117, 1000),
                DecodedEvent::TaskIdentity(TaskIdentityEvent {
                    id: TaskId::new(1),
                    build_path: ":".into(),
                    task_path: ":app:build".into(),
                }),
            ),
            (
                frame(1563, 2000),
                DecodedEvent::TaskStarted(TaskStartedEvent {
                    id: TaskId::new(1),
                    build_path: ":".into(),
                    path: ":app:build".into(),
                    class_name: Some("org.gradle.DefaultTask".into()),
                }),
            ),
            (
                frame(2074, 3000),
                DecodedEvent::TaskFinished(TaskFinishedEvent {
                    id: TaskId::new(1),
                    path: ":app:build".into(),
                    outcome: Some(3),
                    cacheable: Some(false),
                    caching_disabled_reason_category: None,
                    caching_disabled_explanation: None,
                    origin_build_invocation_id: None,
                    origin_build_cache_key: None,
                    origin_execution_time: None,
                    actionable: Some(false),
                    skip_reason_message: None,
                    up_to_date_messages: None,
                }),
            ),
        ];
        let payload = assemble(events);
        assert_eq!(payload.tasks.len(), 1);
        let task = &payload.tasks[0];
        assert_eq!(task.task_path, ":app:build");
        assert_eq!(task.started_at, Some(2000));
        assert_eq!(task.finished_at, Some(3000));
        assert_eq!(task.duration_ms, Some(1000));
        assert!(matches!(task.outcome, Some(TaskOutcome::Success)));
        assert!(task.inputs.is_none());
    }

    #[test]
    fn test_assemble_task_cache_invalidation_fields() {
        let events = vec![
            (
                frame(117, 1000),
                DecodedEvent::TaskIdentity(TaskIdentityEvent {
                    id: TaskId::new(1),
                    build_path: ":".into(),
                    task_path: ":app:compileJava".into(),
                }),
            ),
            (
                frame(1563, 2000),
                DecodedEvent::TaskStarted(TaskStartedEvent {
                    id: TaskId::new(1),
                    build_path: ":".into(),
                    path: ":app:compileJava".into(),
                    class_name: Some("org.gradle.DefaultTask".into()),
                }),
            ),
            (
                frame(2074, 3000),
                DecodedEvent::TaskFinished(TaskFinishedEvent {
                    id: TaskId::new(1),
                    path: ":app:compileJava".into(),
                    outcome: Some(3),
                    cacheable: Some(true),
                    caching_disabled_reason_category: None,
                    caching_disabled_explanation: None,
                    origin_build_invocation_id: Some("abc123invocation".into()),
                    origin_build_cache_key: None,
                    origin_execution_time: None,
                    actionable: Some(true),
                    skip_reason_message: None,
                    up_to_date_messages: Some(vec![
                        "Input property 'foo' has changed".into(),
                        "Input property 'bar' has changed".into(),
                    ]),
                }),
            ),
        ];
        let payload = assemble(events);
        assert_eq!(payload.tasks.len(), 1);
        let task = &payload.tasks[0];
        assert_eq!(
            task.up_to_date_messages,
            Some(vec![
                "Input property 'foo' has changed".to_string(),
                "Input property 'bar' has changed".to_string(),
            ])
        );
        assert_eq!(
            task.origin_build_invocation_id,
            Some("abc123invocation".to_string())
        );
    }
}
