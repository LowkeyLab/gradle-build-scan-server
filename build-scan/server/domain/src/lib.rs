use chrono::{DateTime, Utc};
use uuid::Uuid;

// === ID types ===

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BuildScanId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskId(pub Uuid);

// === String-wrapped value types ===

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BuildToolType(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BuildToolVersion(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PluginVersion(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Hostname(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OsName(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OsVersion(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JvmVendor(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JvmVersion(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskPath(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClassName(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RequestedTask(pub String);

// === Numeric value types ===

/// Epoch milliseconds — a point in time
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Timestamp(pub i64);

/// Duration in milliseconds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Duration(pub i64);

// === Enums (strict, no Unknown fallback) ===

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuildOutcome {
    Success,
    Failed,
    ParseError,
}

impl std::fmt::Display for BuildOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildOutcome::Success => write!(f, "success"),
            BuildOutcome::Failed => write!(f, "failed"),
            BuildOutcome::ParseError => write!(f, "parse_error"),
        }
    }
}

impl std::str::FromStr for BuildOutcome {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "success" => Ok(BuildOutcome::Success),
            "failed" => Ok(BuildOutcome::Failed),
            "parse_error" => Ok(BuildOutcome::ParseError),
            other => Err(format!("unknown build outcome: '{other}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskOutcome {
    UpToDate,
    Skipped,
    Failed,
    Success,
    FromCache,
    NoSource,
    AvoidedForUnknownReason,
}

impl std::fmt::Display for TaskOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskOutcome::UpToDate => write!(f, "UpToDate"),
            TaskOutcome::Skipped => write!(f, "Skipped"),
            TaskOutcome::Failed => write!(f, "Failed"),
            TaskOutcome::Success => write!(f, "Success"),
            TaskOutcome::FromCache => write!(f, "FromCache"),
            TaskOutcome::NoSource => write!(f, "NoSource"),
            TaskOutcome::AvoidedForUnknownReason => write!(f, "AvoidedForUnknownReason"),
        }
    }
}

impl std::str::FromStr for TaskOutcome {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "UpToDate" => Ok(TaskOutcome::UpToDate),
            "Skipped" => Ok(TaskOutcome::Skipped),
            "Failed" => Ok(TaskOutcome::Failed),
            "Success" => Ok(TaskOutcome::Success),
            "FromCache" => Ok(TaskOutcome::FromCache),
            "NoSource" => Ok(TaskOutcome::NoSource),
            "AvoidedForUnknownReason" => Ok(TaskOutcome::AvoidedForUnknownReason),
            other => Err(format!("unknown task outcome: '{other}'")),
        }
    }
}

// === Aggregates ===

#[derive(Debug, Clone)]
pub struct BuildScan {
    pub id: BuildScanId,
    pub build_tool_type: BuildToolType,
    pub build_tool_version: BuildToolVersion,
    pub plugin_version: PluginVersion,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub outcome: Option<BuildOutcome>,
    pub requested_tasks: Option<Vec<RequestedTask>>,
    pub hostname: Option<Hostname>,
    pub os_name: Option<OsName>,
    pub os_version: Option<OsVersion>,
    pub jvm_vendor: Option<JvmVendor>,
    pub jvm_version: Option<JvmVersion>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: TaskId,
    pub scan_id: BuildScanId,
    pub task_path: TaskPath,
    pub class_name: Option<ClassName>,
    pub outcome: Option<TaskOutcome>,
    pub cacheable: Option<bool>,
    pub start_timestamp: Option<Timestamp>,
    pub finish_timestamp: Option<Timestamp>,
    pub cache_key: Option<CacheKey>,
    pub origin_execution_time: Option<Duration>,
}
