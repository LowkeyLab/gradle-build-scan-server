use serde::{Deserialize, Serialize};

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
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
    pub roots: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInputsSnapshottingResultData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implementation: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property_names: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_inputs: Option<i64>,
    pub file_inputs: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedNodeData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub dependencies: Vec<i64>,
    pub must_run_after: Vec<i64>,
    pub should_run_after: Vec<i64>,
    pub finalized_by: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformExecutionRequestData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identification_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_id: Option<i64>,
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
