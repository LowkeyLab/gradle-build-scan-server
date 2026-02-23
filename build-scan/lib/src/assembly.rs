use std::collections::HashMap;

use events::DecodedEvent;
use framing::FramedEvent;
use models::{BuildScanPayload, RawEventSummary, Task, TaskOutcome};

pub fn assemble(events: Vec<(FramedEvent, DecodedEvent)>) -> BuildScanPayload {
    let mut identities: HashMap<i64, (String, String)> = HashMap::new();
    let mut started: HashMap<i64, (String, Option<String>, i64)> = HashMap::new();
    let mut finished: HashMap<i64, FinishedInfo> = HashMap::new();
    let mut raw_counts: HashMap<u16, usize> = HashMap::new();

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
                        actionable: e.actionable,
                        timestamp: frame.timestamp,
                    },
                );
            }
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
                actionable: fin.and_then(|f| f.actionable),
                started_at,
                finished_at,
                duration_ms,
            }
        })
        .collect();

    tasks.sort_by_key(|t| t.id);

    let mut raw_events: Vec<RawEventSummary> = raw_counts
        .into_iter()
        .map(|(wire_id, count)| RawEventSummary { wire_id, count })
        .collect();
    raw_events.sort_by_key(|r| r.wire_id);

    BuildScanPayload { tasks, raw_events }
}

struct FinishedInfo {
    outcome: Option<TaskOutcome>,
    cacheable: Option<bool>,
    caching_disabled_reason: Option<String>,
    caching_disabled_explanation: Option<String>,
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
    }
}
