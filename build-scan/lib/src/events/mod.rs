use std::collections::HashMap;

use error::ParseError;
use models::{ExecutorId, FailureId, FileRefId, TaskId, TransformId};

pub mod basic_memory_stats;
pub mod build_agent;
pub mod build_finished;
pub mod build_modes;
pub mod build_requested_tasks;
pub mod build_started;
pub mod daemon_state;
pub mod encoding;
pub mod file_ref_roots;
pub mod hardware;
pub mod java_toolchain_usage;
pub mod jvm;
pub mod jvm_args;
pub mod locality;
pub mod os;
pub mod output_styled_text_event;
pub mod planned_node;
pub mod resource_usage;
pub mod scope_ids;
pub mod task_finished;
pub mod task_identity;
pub mod task_inputs_file_property;
pub mod task_inputs_file_property_root;
pub mod task_inputs_implementation;
pub mod task_inputs_property_names;
pub mod task_inputs_snapshotting_finished;
pub mod task_inputs_snapshotting_started;
pub mod task_inputs_value_properties;
pub mod task_registration_summary;
pub mod task_started;
pub mod test_case;
pub mod test_executor_finished;
pub mod test_executor_identity;
pub mod test_executor_started;
pub mod test_result;
pub mod transform_execution_finished;
pub mod transform_execution_request;
pub mod transform_execution_started;
pub mod transform_identification;

pub trait BodyDecoder: Send + Sync {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError>;
}

#[derive(Debug, Clone)]
pub enum DecodedEvent {
    TaskIdentity(TaskIdentityEvent),
    TaskStarted(TaskStartedEvent),
    TaskFinished(TaskFinishedEvent),
    TaskInputsSnapshottingStarted(TaskInputsSnapshottingStartedEvent),
    TransformExecutionRequest(TransformExecutionRequestEvent),
    PlannedNode(PlannedNodeEvent),
    TaskInputsValueProperties(TaskInputsValuePropertiesEvent),
    TaskInputsPropertyNames(TaskInputsPropertyNamesEvent),
    TaskInputsImplementation(TaskInputsImplementationEvent),
    TaskInputsFileProperty(TaskInputsFilePropertyEvent),
    TaskInputsSnapshottingFinished(TaskInputsSnapshottingFinishedEvent),
    TaskInputsFilePropertyRoot(TaskInputsFilePropertyRootEvent),
    JavaToolchainUsage(JavaToolchainUsageEvent),
    TransformExecutionStarted(TransformExecutionStartedEvent),
    TransformIdentification(TransformIdentificationEvent),
    TransformExecutionFinished(TransformExecutionFinishedEvent),
    OutputStyledText(OutputStyledTextEvent),
    BuildStarted,
    BuildAgent(BuildAgentEvent),
    BuildRequestedTasks(BuildRequestedTasksEvent),
    BuildFinished(BuildFinishedEvent),
    BuildModes(BuildModesEvent),
    DaemonState(DaemonStateEvent),
    Encoding(EncodingEvent),
    FileRefRoots(FileRefRootsEvent),
    Hardware(HardwareEvent),
    Jvm(JvmEvent),
    JvmArgs(JvmArgsEvent),
    Locality(LocalityEvent),
    Os(OsEvent),
    ScopeIds(ScopeIdsEvent),
    TaskRegistrationSummary(TaskRegistrationSummaryEvent),
    BasicMemoryStats(BasicMemoryStatsEvent),
    ResourceUsage(ResourceUsageEvent),
    TestCase(TestCaseEvent),
    TestExecutorIdentity(TestExecutorIdentityEvent),
    TestExecutorStarted(TestExecutorStartedEvent),
    TestExecutorFinished(TestExecutorFinishedEvent),
    TestResult(TestResultEvent),
    Raw(RawEvent),
}

#[derive(Debug, Clone)]
pub struct TaskIdentityEvent {
    pub id: TaskId,
    pub build_path: String,
    pub task_path: String,
}

#[derive(Debug, Clone)]
pub struct TaskStartedEvent {
    pub id: TaskId,
    pub build_path: String,
    pub path: String,
    pub class_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TaskFinishedEvent {
    pub id: TaskId,
    pub path: String,
    pub outcome: Option<u64>,
    pub cacheable: Option<bool>,
    pub caching_disabled_reason_category: Option<String>,
    pub caching_disabled_explanation: Option<String>,
    pub origin_build_invocation_id: Option<String>,
    pub origin_build_cache_key: Option<Vec<u8>>,
    pub actionable: Option<bool>,
    pub skip_reason_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TaskInputsSnapshottingStartedEvent {
    pub task: TaskId,
}

#[derive(Debug, Clone)]
pub struct TransformExecutionRequestEvent {
    pub node_id: Option<TaskId>,
    pub identification_id: Option<TransformId>,
    pub execution_id: Option<TransformId>,
}

#[derive(Debug, Clone)]
pub struct PlannedNodeEvent {
    pub id: Option<TaskId>,
    pub dependencies: Vec<TaskId>,
    pub must_run_after: Vec<TaskId>,
    pub should_run_after: Vec<TaskId>,
    pub finalized_by: Vec<TaskId>,
}

#[derive(Debug, Clone)]
pub struct TaskInputsValuePropertiesEvent {
    pub id: Option<TaskId>,
    pub hashes: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct TaskInputsPropertyNamesEvent {
    pub id: Option<TaskId>,
    pub value_inputs: Vec<String>,
    pub file_inputs: Vec<String>,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TaskInputsImplementationEvent {
    pub id: Option<TaskId>,
    pub class_loader_hash: Option<Vec<u8>>,
    pub action_class_loader_hashes: Vec<Vec<u8>>,
    pub action_class_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TaskInputsFilePropertyEvent {
    pub id: Option<TaskId>,
    pub attributes: Vec<String>,
    pub hash: Option<Vec<u8>>,
    pub roots: Vec<FileRefId>,
}

#[derive(Debug, Clone)]
pub struct TaskInputsSnapshottingFinishedEvent {
    pub task: Option<TaskId>,
    pub result: Option<TaskInputsSnapshottingResult>,
    pub failure_id: Option<FailureId>,
}

#[derive(Debug, Clone)]
pub struct TaskInputsSnapshottingResult {
    pub hash: Option<Vec<u8>>,
    pub implementation: Option<TaskId>,
    pub property_names: Option<TaskId>,
    pub value_inputs: Option<TaskId>,
    pub file_inputs: Vec<TaskId>,
}

#[derive(Debug, Clone)]
pub struct TaskInputsFilePropertyRootEvent {
    pub id: Option<TaskId>,
    pub file: FileRef,
    pub root_hash: Option<Vec<u8>>,
    pub children: Vec<FilePropertyRootChild>,
}

#[derive(Debug, Clone)]
pub struct FileRef {
    pub root: Option<u64>,
    pub path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FilePropertyRootChild {
    pub name: Option<String>,
    pub hash: Option<Vec<u8>>,
    pub parent: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct JavaToolchainUsageEvent {
    pub task_id: TaskId,
    pub toolchain_id: i64,
    pub tool_name: String,
}

#[derive(Debug, Clone)]
pub struct TransformExecutionStartedEvent {
    pub id: TransformId,
}

#[derive(Debug, Clone)]
pub struct TransformIdentificationEvent {
    pub id: TransformId,
    pub component_identity: i32,
    pub input_artifact_name: String,
    pub transform_action_class: String,
    pub from_attributes: Vec<i32>,
    pub to_attributes: Vec<i32>,
}

#[derive(Debug, Clone)]
pub struct TransformExecutionFinishedEvent {
    pub id: TransformId,
    pub failure_id: Option<FailureId>,
    pub outcome: Option<u64>,
    pub execution_reasons: Vec<String>,
    pub caching_disabled_reason_category: Option<String>,
    pub caching_disabled_explanation: Option<String>,
    pub origin_build_invocation_id: Option<String>,
    pub origin_build_cache_key: Option<Vec<u8>>,
    pub origin_execution_time: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct OutputStyledTextEvent {
    pub category: Option<String>,
    pub log_level: Option<String>,
    pub spans: Vec<OutputSpan>,
    pub owner_type: Option<u64>,
    pub owner_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OutputSpan {
    pub text: String,
    pub style: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BuildAgentEvent {
    pub username: Option<String>,
    pub local_hostname: Option<String>,
    pub public_hostname: Option<String>,
    pub ip_addresses: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BuildRequestedTasksEvent {
    pub requested: Vec<String>,
    pub excluded: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BuildFinishedEvent {
    pub failure_id: Option<FailureId>,
}

#[derive(Debug, Clone)]
pub struct BuildModesEvent {
    pub refresh_dependencies: bool,
    pub parallel_project_execution: bool,
    pub rerun_tasks: bool,
    pub continuous: bool,
    pub continue_on_failure: bool,
    pub configure_on_demand: bool,
    pub daemon: bool,
    pub offline: bool,
    pub dry_run: bool,
    pub max_workers: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct DaemonStateEvent {
    pub start_time: Option<i64>,
    pub build_number: Option<i32>,
    pub number_of_running_daemons: Option<i32>,
    pub idle_timeout: Option<i64>,
    pub single_use: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct EncodingEvent {
    pub default_charset: String,
}

#[derive(Debug, Clone)]
pub struct FileRefRootsEvent {
    pub entries: Vec<FileRefRootEntry>,
}

#[derive(Debug, Clone)]
pub struct FileRefRootEntry {
    pub root_type: u64,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct HardwareEvent {
    pub num_processors: i32,
}

#[derive(Debug, Clone)]
pub struct JvmEvent {
    pub version: Option<String>,
    pub vendor: Option<String>,
    pub runtime_name: Option<String>,
    pub runtime_version: Option<String>,
    pub class_version: Option<String>,
    pub vm_info: Option<String>,
    pub vm_name: Option<String>,
    pub vm_version: Option<String>,
    pub vm_vendor: Option<String>,
}

#[derive(Debug, Clone)]
pub struct JvmArgsEvent {
    pub effective: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LocalityEvent {
    pub locale_language: Option<String>,
    pub locale_country: Option<String>,
    pub locale_variant: Option<String>,
    pub time_zone_id: Option<String>,
    pub time_zone_offset_millis: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct OsEvent {
    pub family: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub arch: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScopeIdsEvent {
    pub build_invocation_id: Option<String>,
    pub workspace_id: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TaskRegistrationSummaryEvent {
    pub task_count: i32,
}

#[derive(Debug, Clone)]
pub struct BasicMemoryStatsEvent {
    pub free: Option<i64>,
    pub total: Option<i64>,
    pub max: Option<i64>,
    pub peak_snapshots: Vec<MemoryPoolSnapshotEvent>,
    pub gc_time: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct MemoryPoolSnapshotEvent {
    pub name: Option<String>,
    pub heap: bool,
    pub init: Option<i64>,
    pub used: Option<i64>,
    pub committed: Option<i64>,
    pub max: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ResourceUsageEvent {
    pub timestamps: Vec<Vec<u8>>,
    pub build_process_cpu: NormalizedSamplesEvent,
    pub build_child_processes_cpu: NormalizedSamplesEvent,
    pub all_processes_cpu_sum: NormalizedSamplesEvent,
    pub all_processes_cpu: Option<Vec<u8>>,
    pub build_process_memory: NormalizedSamplesEvent,
    pub build_child_processes_memory: NormalizedSamplesEvent,
    pub all_processes_memory: NormalizedSamplesEvent,
    pub total_system_memory: Option<i64>,
    pub disk_read_speed: NormalizedSamplesEvent,
    pub disk_write_speed: NormalizedSamplesEvent,
    pub network_download_speed: NormalizedSamplesEvent,
    pub network_upload_speed: NormalizedSamplesEvent,
    pub processes: Vec<ProcessEvent>,
    pub top_processes_by_cpu: IndexedNormalizedSamplesEvent,
    pub top_processes_by_memory: IndexedNormalizedSamplesEvent,
}

#[derive(Debug, Clone, Default)]
pub struct NormalizedSamplesEvent {
    pub samples: Option<Vec<u8>>,
    pub max: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct IndexedNormalizedSamplesEvent {
    pub indices: Vec<Vec<i32>>,
    pub samples: Vec<Vec<u8>>,
    pub max: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ProcessEvent {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub process_type: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct TestCaseEvent {
    pub executor_id: ExecutorId,
    pub class_name: String,
    pub method_name: Option<String>,
    pub executor_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TestExecutorIdentityEvent {
    pub executor_id: ExecutorId,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct TestExecutorStartedEvent {
    pub executor_id: ExecutorId,
}

#[derive(Debug, Clone)]
pub struct TestExecutorFinishedEvent {
    pub executor_id: ExecutorId,
}

#[derive(Debug, Clone)]
pub struct TestResultEvent {
    pub task: i64,
    pub id: i64,
    pub failed: bool,
    pub skipped: bool,
}

#[derive(Debug, Clone)]
pub struct RawEvent {
    pub wire_id: u16,
    pub body: Vec<u8>,
}

pub struct DecoderRegistry {
    decoders: HashMap<u16, Box<dyn BodyDecoder>>,
}

impl Default for DecoderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DecoderRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            decoders: HashMap::new(),
        };
        registry.register(2, Box::new(build_agent::BuildAgentDecoder));
        registry.register(
            5,
            Box::new(build_requested_tasks::BuildRequestedTasksDecoder),
        );
        registry.register(6, Box::new(build_started::BuildStartedDecoder));
        registry.register(12, Box::new(hardware::HardwareDecoder));
        registry.register(13, Box::new(jvm_args::JvmArgsDecoder));
        registry.register(14, Box::new(jvm::JvmDecoder));
        registry.register(15, Box::new(locality::LocalityDecoder));
        registry.register(16, Box::new(os::OsDecoder));
        registry.register(39, Box::new(scope_ids::ScopeIdsDecoder));
        registry.register(49, Box::new(file_ref_roots::FileRefRootsDecoder));
        registry.register(56, Box::new(encoding::EncodingDecoder));
        registry.register(
            88,
            Box::new(task_inputs_file_property_root::TaskInputsFilePropertyRootDecoder),
        );
        registry.register(
            91,
            Box::new(task_inputs_implementation::TaskInputsImplementationDecoder),
        );
        registry.register(
            92,
            Box::new(task_inputs_property_names::TaskInputsPropertyNamesDecoder),
        );
        registry.register(
            94,
            Box::new(task_inputs_snapshotting_started::TaskInputsSnapshottingStartedDecoder),
        );
        registry.register(
            95,
            Box::new(task_inputs_value_properties::TaskInputsValuePropertiesDecoder),
        );
        registry.register(
            115,
            Box::new(java_toolchain_usage::JavaToolchainUsageDecoder),
        );
        registry.register(117, Box::new(task_identity::TaskIdentityDecoder));
        registry.register(119, Box::new(planned_node::PlannedNodeDecoder));
        registry.register(
            122,
            Box::new(task_registration_summary::TaskRegistrationSummaryDecoder),
        );
        registry.register(
            136,
            Box::new(transform_identification::TransformIdentificationDecoder),
        );
        registry.register(
            137,
            Box::new(transform_execution_request::TransformExecutionRequestDecoder),
        );
        registry.register(
            138,
            Box::new(transform_execution_started::TransformExecutionStartedDecoder),
        );
        registry.register(
            345,
            Box::new(task_inputs_file_property::TaskInputsFilePropertyDecoder),
        );
        registry.register(
            349,
            Box::new(task_inputs_snapshotting_finished::TaskInputsSnapshottingFinishedDecoder),
        );
        registry.register(257, Box::new(basic_memory_stats::BasicMemoryStatsDecoder));
        registry.register(407, Box::new(resource_usage::ResourceUsageDecoder));
        registry.register(259, Box::new(build_finished::BuildFinishedDecoder));
        registry.register(265, Box::new(daemon_state::DaemonStateDecoder));
        registry.register(
            274,
            Box::new(output_styled_text_event::OutputStyledTextEventDecoder),
        );
        registry.register(
            395,
            Box::new(transform_execution_finished::TransformExecutionFinishedDecoder),
        );
        registry.register(516, Box::new(build_modes::BuildModesDecoder));
        registry.register(1563, Box::new(task_started::TaskStartedDecoder));
        registry.register(2074, Box::new(task_finished::TaskFinishedDecoder));
        registry.register(
            127,
            Box::new(test_executor_identity::TestExecutorIdentityDecoder),
        );
        registry.register(
            128,
            Box::new(test_executor_started::TestExecutorStartedDecoder),
        );
        registry.register(284, Box::new(test_result::TestResultDecoder));
        registry.register(798, Box::new(test_case::TestCaseDecoder));
        registry.register(
            803,
            Box::new(test_executor_finished::TestExecutorFinishedDecoder),
        );
        registry
    }

    pub fn register(&mut self, wire_id: u16, decoder: Box<dyn BodyDecoder>) {
        self.decoders.insert(wire_id, decoder);
    }

    pub fn decode(&self, wire_id: u16, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        match self.decoders.get(&wire_id) {
            Some(decoder) => match decoder.decode(body) {
                Ok(event) => Ok(event),
                Err(e) => {
                    let hex_preview: String = body
                        .iter()
                        .take(64)
                        .map(|b| format!("{:02x}", b))
                        .collect::<Vec<_>>()
                        .join(" ");
                    tracing::warn!(
                        wire_id,
                        error = %e,
                        body_len = body.len(),
                        body_hex = %hex_preview,
                        "Failed to decode event, treating as raw"
                    );
                    Ok(DecodedEvent::Raw(RawEvent {
                        wire_id,
                        body: body.to_vec(),
                    }))
                }
            },
            None => Ok(DecodedEvent::Raw(RawEvent {
                wire_id,
                body: body.to_vec(),
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_unknown_wire_id_returns_raw_with_original_bytes() {
        let registry = DecoderRegistry::new();
        let unknown_id: u16 = 9999;
        let body = vec![0xDE, 0xAD, 0xBE, 0xEF];

        let result = registry.decode(unknown_id, &body).unwrap();
        match result {
            DecodedEvent::Raw(raw) => {
                assert_eq!(raw.wire_id, unknown_id);
                assert_eq!(raw.body, body);
            }
            other => panic!("expected Raw, got {:?}", other),
        }
    }

    #[test]
    fn decode_registered_wire_id_with_invalid_body_falls_back_to_raw() {
        let registry = DecoderRegistry::new();
        // wire_id 259 = BuildFinished, which expects at least a flags byte.
        // An empty body will cause the decoder to fail.
        let build_finished_id: u16 = 259;
        let invalid_body: Vec<u8> = vec![];

        let result = registry.decode(build_finished_id, &invalid_body).unwrap();
        match result {
            DecodedEvent::Raw(raw) => {
                assert_eq!(raw.wire_id, build_finished_id);
                assert_eq!(raw.body, invalid_body);
            }
            other => panic!("expected Raw fallback for invalid body, got {:?}", other),
        }
    }

    #[test]
    fn decode_registered_wire_id_with_truncated_body_falls_back_to_raw() {
        let registry = DecoderRegistry::new();
        // wire_id 2074 = TaskFinished, expects a 2-byte BE flags prefix + more data.
        // Give it only 1 byte to force an UnexpectedEof.
        let task_finished_id: u16 = 2074;
        let truncated_body = vec![0x00];

        let result = registry.decode(task_finished_id, &truncated_body).unwrap();
        match result {
            DecodedEvent::Raw(raw) => {
                assert_eq!(raw.wire_id, task_finished_id);
                assert_eq!(raw.body, truncated_body);
            }
            other => panic!("expected Raw fallback for truncated body, got {:?}", other),
        }
    }
}
