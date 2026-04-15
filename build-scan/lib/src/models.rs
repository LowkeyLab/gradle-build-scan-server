use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TaskId(pub(crate) i64);

impl TaskId {
    pub fn new(id: i64) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TransformId(pub(crate) i64);

impl TransformId {
    pub fn new(id: i64) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExecutorId(pub(crate) i64);

impl ExecutorId {
    pub fn new(id: i64) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FailureId(pub(crate) i64);

impl FailureId {
    pub fn new(id: i64) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FileRefId(pub(crate) i64);

impl FileRefId {
    pub fn new(id: i64) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildScanPayload {
    pub tasks: Vec<Task>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub planned_nodes: Vec<PlannedNodeData>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub transform_execution_requests: Vec<TransformExecutionRequestData>,
    pub raw_events: Vec<RawEventSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_registration_summary: Option<TaskRegistrationSummaryData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub basic_memory_stats: Option<BasicMemoryStatsData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_usage: Option<ResourceUsageData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jvm_vendor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jvm_version: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub requested_tasks: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tests: Vec<TestCase>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CacheOperationType {
    LocalLoad,
    RemoteLoad,
    Pack,
    Unpack,
    LocalStore,
    RemoteStore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheOperation {
    pub operation_type: CacheOperationType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hit: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stored: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive_entry_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_cache_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejected_reason: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub build_path: String,
    pub task_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<TaskOutcome>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cacheable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caching_disabled_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caching_disabled_explanation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_build_cache_key: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actionable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<TaskInputs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub up_to_date_messages: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_build_invocation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_execution_time: Option<i64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cache_operations: Vec<CacheOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskOutcome {
    UpToDate,
    Skipped,
    Failed,
    Success,
    FromCache,
    NoSource,
    AvoidedForUnknownReason,
}

impl TaskOutcome {
    pub fn from_ordinal(ordinal: u64) -> Option<Self> {
        match ordinal {
            0 => Some(Self::UpToDate),
            1 => Some(Self::Skipped),
            2 => Some(Self::Failed),
            3 => Some(Self::Success),
            4 => Some(Self::FromCache),
            5 => Some(Self::NoSource),
            6 => Some(Self::AvoidedForUnknownReason),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawEventSummary {
    pub wire_id: u16,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskInputs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property_names: Option<TaskInputsPropertyNamesData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implementation: Option<TaskInputsImplementationData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_properties: Option<TaskInputsValuePropertiesData>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub file_property_roots: Vec<TaskInputsFilePropertyRootData>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub file_properties: Vec<TaskInputsFilePropertyData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshotting_result: Option<TaskInputsSnapshottingResultData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInputsPropertyNamesData {
    pub value_inputs: Vec<String>,
    pub file_inputs: Vec<String>,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInputsImplementationData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_loader_hash: Option<Vec<u8>>,
    pub action_class_loader_hashes: Vec<Vec<u8>>,
    pub action_class_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInputsValuePropertiesData {
    pub hashes: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInputsFilePropertyRootData {
    pub file_root: Option<u64>,
    pub file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_hash: Option<Vec<u8>>,
    pub children: Vec<FilePropertyRootChildData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePropertyRootChildData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInputsFilePropertyData {
    pub attributes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<Vec<u8>>,
    pub roots: Vec<FileRefId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInputsSnapshottingResultData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implementation: Option<TaskId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property_names: Option<TaskId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_inputs: Option<TaskId>,
    pub file_inputs: Vec<TaskId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedNodeData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<TaskId>,
    pub dependencies: Vec<TaskId>,
    pub must_run_after: Vec<TaskId>,
    pub should_run_after: Vec<TaskId>,
    pub finalized_by: Vec<TaskId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformExecutionRequestData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<TaskId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identification_id: Option<TransformId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_id: Option<TransformId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRegistrationSummaryData {
    pub task_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicMemoryStatsData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub free: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub peak_snapshots: Vec<MemoryPoolSnapshotData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gc_time: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPoolSnapshotData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub heap: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub committed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsageData {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub timestamps: Vec<Vec<u8>>,
    pub build_process_cpu: NormalizedSamplesData,
    pub build_child_processes_cpu: NormalizedSamplesData,
    pub all_processes_cpu_sum: NormalizedSamplesData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_processes_cpu: Option<Vec<u8>>,
    pub build_process_memory: NormalizedSamplesData,
    pub build_child_processes_memory: NormalizedSamplesData,
    pub all_processes_memory: NormalizedSamplesData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_system_memory: Option<i64>,
    pub disk_read_speed: NormalizedSamplesData,
    pub disk_write_speed: NormalizedSamplesData,
    pub network_download_speed: NormalizedSamplesData,
    pub network_upload_speed: NormalizedSamplesData,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub processes: Vec<ProcessData>,
    pub top_processes_by_cpu: IndexedNormalizedSamplesData,
    pub top_processes_by_memory: IndexedNormalizedSamplesData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedSamplesData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub samples: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedNormalizedSamplesData {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub indices: Vec<Vec<i32>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub samples: Vec<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestOutcome {
    Passed,
    Failed,
    Skipped,
}

impl TestOutcome {
    pub fn from_ordinal(ordinal: u64) -> Option<Self> {
        match ordinal {
            0 => Some(Self::Passed),
            1 => Some(Self::Failed),
            2 => Some(Self::Skipped),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub class_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executor_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<TestOutcome>,
}
