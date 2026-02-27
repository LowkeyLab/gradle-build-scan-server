use std::collections::HashMap;

use events::DecodedEvent;
use framing::FramedEvent;
use models::{BuildScanPayload, RawEventSummary, Task, TaskOutcome};

pub fn assemble(events: Vec<(FramedEvent, DecodedEvent)>) -> BuildScanPayload {
    let mut identities: HashMap<i64, (String, String)> = HashMap::new();
    let mut started: HashMap<i64, (String, Option<String>, i64)> = HashMap::new();
    let mut finished: HashMap<i64, FinishedInfo> = HashMap::new();
    let mut raw_counts: HashMap<u16, usize> = HashMap::new();
    let mut property_names_map: HashMap<i64, events::TaskInputsPropertyNamesEvent> = HashMap::new();
    let mut implementation_map: HashMap<i64, events::TaskInputsImplementationEvent> =
        HashMap::new();
    let mut value_properties_map: HashMap<i64, events::TaskInputsValuePropertiesEvent> =
        HashMap::new();
    let mut file_property_roots_map: HashMap<i64, Vec<events::TaskInputsFilePropertyRootEvent>> =
        HashMap::new();
    let mut file_properties_map: HashMap<i64, Vec<events::TaskInputsFilePropertyEvent>> =
        HashMap::new();
    let mut snapshotting_finished_map: HashMap<i64, events::TaskInputsSnapshottingFinishedEvent> =
        HashMap::new();
    let mut planned_nodes: Vec<events::PlannedNodeEvent> = Vec::new();
    let mut transform_requests: Vec<events::TransformExecutionRequestEvent> = Vec::new();
    let mut task_registration_summary: Option<events::TaskRegistrationSummaryEvent> = None;
    let mut basic_memory_stats: Option<events::BasicMemoryStatsEvent> = None;

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
                        actionable: e.actionable,
                        timestamp: frame.timestamp,
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
                task_registration_summary = Some(e.clone());
            }
            DecodedEvent::BasicMemoryStats(e) => {
                basic_memory_stats = Some(e.clone());
            }
            // Decoded for protocol coverage; not yet consumed by assembly.
            DecodedEvent::JavaToolchainUsage(_) => {}
            DecodedEvent::TransformExecutionStarted(_) => {}
            DecodedEvent::TransformIdentification(_) => {}
            DecodedEvent::TransformExecutionFinished(_) => {}
            DecodedEvent::OutputStyledText(_) => {}
            DecodedEvent::BuildStarted => {}
            DecodedEvent::BuildAgent(_) => {}
            DecodedEvent::BuildRequestedTasks(_) => {}
            DecodedEvent::BuildFinished(_) => {}
            DecodedEvent::BuildModes(_) => {}
            DecodedEvent::DaemonState(_) => {}
            DecodedEvent::Encoding(_) => {}
            DecodedEvent::FileRefRoots(_) => {}
            DecodedEvent::Hardware(_) => {}
            DecodedEvent::Jvm(_) => {}
            DecodedEvent::JvmArgs(_) => {}
            DecodedEvent::Locality(_) => {}
            DecodedEvent::Os(_) => {}
            DecodedEvent::ScopeIds(_) => {}
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
    }
}

struct FinishedInfo {
    outcome: Option<TaskOutcome>,
    cacheable: Option<bool>,
    caching_disabled_reason: Option<String>,
    caching_disabled_explanation: Option<String>,
    origin_build_cache_key: Option<Vec<u8>>,
    actionable: Option<bool>,
    timestamp: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use events::*;

    fn frame(wire_id: u16, ts: i64) -> FramedEvent {
        FramedEvent {
            wire_id,
            timestamp: ts,
            ordinal: 0,
            body: vec![],
        }
    }

    #[test]
    fn test_assemble_single_task() {
        let events = vec![
            (
                frame(117, 1000),
                DecodedEvent::TaskIdentity(TaskIdentityEvent {
                    id: 1,
                    build_path: ":".into(),
                    task_path: ":app:build".into(),
                }),
            ),
            (
                frame(1563, 2000),
                DecodedEvent::TaskStarted(TaskStartedEvent {
                    id: 1,
                    build_path: ":".into(),
                    path: ":app:build".into(),
                    class_name: Some("org.gradle.DefaultTask".into()),
                }),
            ),
            (
                frame(2074, 3000),
                DecodedEvent::TaskFinished(TaskFinishedEvent {
                    id: 1,
                    path: ":app:build".into(),
                    outcome: Some(3),
                    cacheable: Some(false),
                    caching_disabled_reason_category: None,
                    caching_disabled_explanation: None,
                    origin_build_invocation_id: None,
                    origin_build_cache_key: None,
                    actionable: Some(false),
                    skip_reason_message: None,
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
}
