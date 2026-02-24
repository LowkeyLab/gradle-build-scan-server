use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildScanPayload {
    pub tasks: Vec<Task>,
    pub raw_events: Vec<RawEventSummary>,
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
