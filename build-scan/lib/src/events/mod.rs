use std::collections::HashMap;

use error::ParseError;

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
    Raw(RawEvent),
}

#[derive(Debug, Clone)]
pub struct TaskIdentityEvent {
    pub id: i64,
    pub build_path: String,
    pub task_path: String,
}

#[derive(Debug, Clone)]
pub struct TaskStartedEvent {
    pub id: i64,
    pub build_path: String,
    pub path: String,
    pub class_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TaskFinishedEvent {
    pub id: i64,
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
    pub task: i64,
}

#[derive(Debug, Clone)]
pub struct TransformExecutionRequestEvent {
    pub node_id: Option<i64>,
    pub identification_id: Option<i64>,
    pub execution_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct PlannedNodeEvent {
    pub id: Option<i64>,
    pub dependencies: Vec<i64>,
    pub must_run_after: Vec<i64>,
    pub should_run_after: Vec<i64>,
    pub finalized_by: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct TaskInputsValuePropertiesEvent {
    pub id: Option<i64>,
    pub hashes: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct TaskInputsPropertyNamesEvent {
    pub id: Option<i64>,
    pub value_inputs: Vec<String>,
    pub file_inputs: Vec<String>,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TaskInputsImplementationEvent {
    pub id: Option<i64>,
    pub class_loader_hash: Option<Vec<u8>>,
    pub action_class_loader_hashes: Vec<Vec<u8>>,
    pub action_class_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TaskInputsFilePropertyEvent {
    pub id: Option<i64>,
    pub attributes: Vec<String>,
    pub hash: Option<Vec<u8>>,
    pub roots: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct TaskInputsSnapshottingFinishedEvent {
    pub task: Option<i64>,
    pub result: Option<TaskInputsSnapshottingResult>,
    pub failure_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct TaskInputsSnapshottingResult {
    pub hash: Option<Vec<u8>>,
    pub implementation: Option<i64>,
    pub property_names: Option<i64>,
    pub value_inputs: Option<i64>,
    pub file_inputs: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct TaskInputsFilePropertyRootEvent {
    pub id: Option<i64>,
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
    pub task_id: i64,
    pub toolchain_id: i64,
    pub tool_name: String,
}

#[derive(Debug, Clone)]
pub struct TransformExecutionStartedEvent {
    pub id: i64,
}

#[derive(Debug, Clone)]
pub struct TransformIdentificationEvent {
    pub id: i64,
    pub component_identity: i32,
    pub input_artifact_name: String,
    pub transform_action_class: String,
    pub from_attributes: Vec<i32>,
    pub to_attributes: Vec<i32>,
}

#[derive(Debug, Clone)]
pub struct TransformExecutionFinishedEvent {
    pub id: i64,
    pub failure_id: Option<i64>,
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
    pub failure_id: Option<i64>,
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
pub struct RawEvent {
    pub wire_id: u16,
    pub body: Vec<u8>,
}

pub struct DecoderRegistry {
    decoders: HashMap<u16, Box<dyn BodyDecoder>>,
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
        registry
    }

    pub fn register(&mut self, wire_id: u16, decoder: Box<dyn BodyDecoder>) {
        self.decoders.insert(wire_id, decoder);
    }

    pub fn decode(&self, wire_id: u16, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        match self.decoders.get(&wire_id) {
            Some(decoder) => decoder.decode(body),
            None => Ok(DecodedEvent::Raw(RawEvent {
                wire_id,
                body: body.to_vec(),
            })),
        }
    }
}
