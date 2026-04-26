use chrono::{DateTime, Utc};
use derive_builder::Builder;
use uuid::Uuid;

// === ID types ===

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BuildScanId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TestId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheOperationId(pub Uuid);

impl From<Uuid> for TestId {
    fn from(v: Uuid) -> Self {
        Self(v)
    }
}

impl From<Uuid> for CacheOperationId {
    fn from(v: Uuid) -> Self {
        Self(v)
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TestClassName(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodName(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExecutorName(pub String);

// === Numeric value types ===

/// Epoch milliseconds — a point in time
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Timestamp(pub i64);

/// Duration in milliseconds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Duration(pub i64);

/// Cache archive size in bytes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArchiveSize(pub i64);

/// Test count (passed/failed/skipped totals in a test summary)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TestCount(pub i64);

// === From impls for newtype wrappers (enables derive_builder setter(into)) ===

impl From<Uuid> for BuildScanId {
    fn from(v: Uuid) -> Self {
        Self(v)
    }
}
impl From<Uuid> for TaskId {
    fn from(v: Uuid) -> Self {
        Self(v)
    }
}
impl From<String> for BuildToolType {
    fn from(v: String) -> Self {
        Self(v)
    }
}
impl From<String> for BuildToolVersion {
    fn from(v: String) -> Self {
        Self(v)
    }
}
impl From<String> for PluginVersion {
    fn from(v: String) -> Self {
        Self(v)
    }
}
impl From<String> for Hostname {
    fn from(v: String) -> Self {
        Self(v)
    }
}
impl From<String> for OsName {
    fn from(v: String) -> Self {
        Self(v)
    }
}
impl From<String> for OsVersion {
    fn from(v: String) -> Self {
        Self(v)
    }
}
impl From<String> for JvmVendor {
    fn from(v: String) -> Self {
        Self(v)
    }
}
impl From<String> for JvmVersion {
    fn from(v: String) -> Self {
        Self(v)
    }
}
impl From<String> for TaskPath {
    fn from(v: String) -> Self {
        Self(v)
    }
}
impl From<String> for ClassName {
    fn from(v: String) -> Self {
        Self(v)
    }
}
impl From<String> for CacheKey {
    fn from(v: String) -> Self {
        Self(v)
    }
}
impl From<String> for RequestedTask {
    fn from(v: String) -> Self {
        Self(v)
    }
}
impl From<String> for TestClassName {
    fn from(v: String) -> Self {
        Self(v)
    }
}
impl From<String> for MethodName {
    fn from(v: String) -> Self {
        Self(v)
    }
}
impl From<String> for ExecutorName {
    fn from(v: String) -> Self {
        Self(v)
    }
}
impl From<i64> for Timestamp {
    fn from(v: i64) -> Self {
        Self(v)
    }
}
impl From<i64> for Duration {
    fn from(v: i64) -> Self {
        Self(v)
    }
}
impl From<i64> for ArchiveSize {
    fn from(v: i64) -> Self {
        Self(v)
    }
}
impl From<i64> for TestCount {
    fn from(v: i64) -> Self {
        Self(v)
    }
}

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

#[derive(Debug, Clone, PartialEq)]
pub enum CacheOperationType {
    LocalLoad,
    RemoteLoad,
    Pack,
    Unpack,
    LocalStore,
    RemoteStore,
}

impl std::fmt::Display for CacheOperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LocalLoad => write!(f, "LocalLoad"),
            Self::RemoteLoad => write!(f, "RemoteLoad"),
            Self::Pack => write!(f, "Pack"),
            Self::Unpack => write!(f, "Unpack"),
            Self::LocalStore => write!(f, "LocalStore"),
            Self::RemoteStore => write!(f, "RemoteStore"),
        }
    }
}

impl std::str::FromStr for CacheOperationType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "LocalLoad" => Ok(Self::LocalLoad),
            "RemoteLoad" => Ok(Self::RemoteLoad),
            "Pack" => Ok(Self::Pack),
            "Unpack" => Ok(Self::Unpack),
            "LocalStore" => Ok(Self::LocalStore),
            "RemoteStore" => Ok(Self::RemoteStore),
            _ => Err(format!("unknown cache operation type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TestOutcome {
    Passed,
    Failed,
    Skipped,
}

impl std::fmt::Display for TestOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestOutcome::Passed => write!(f, "Passed"),
            TestOutcome::Failed => write!(f, "Failed"),
            TestOutcome::Skipped => write!(f, "Skipped"),
        }
    }
}

impl std::str::FromStr for TestOutcome {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Passed" => Ok(TestOutcome::Passed),
            "Failed" => Ok(TestOutcome::Failed),
            "Skipped" => Ok(TestOutcome::Skipped),
            other => Err(format!("unknown test outcome: '{other}'")),
        }
    }
}

// === Aggregates ===

#[derive(Debug, Clone, Builder)]
#[builder(setter(into))]
pub struct BuildScan {
    pub id: BuildScanId,
    pub build_tool_type: BuildToolType,
    pub build_tool_version: BuildToolVersion,
    pub plugin_version: PluginVersion,
    #[builder(setter(strip_option), default)]
    pub started_at: Option<DateTime<Utc>>,
    #[builder(setter(strip_option), default)]
    pub finished_at: Option<DateTime<Utc>>,
    #[builder(setter(strip_option), default)]
    pub outcome: Option<BuildOutcome>,
    #[builder(setter(strip_option), default)]
    pub requested_tasks: Option<Vec<RequestedTask>>,
    #[builder(setter(strip_option), default)]
    pub hostname: Option<Hostname>,
    #[builder(setter(strip_option), default)]
    pub os_name: Option<OsName>,
    #[builder(setter(strip_option), default)]
    pub os_version: Option<OsVersion>,
    #[builder(setter(strip_option), default)]
    pub jvm_vendor: Option<JvmVendor>,
    #[builder(setter(strip_option), default)]
    pub jvm_version: Option<JvmVersion>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Builder)]
#[builder(setter(into))]
pub struct Task {
    pub id: TaskId,
    pub scan_id: BuildScanId,
    pub task_path: TaskPath,
    #[builder(setter(strip_option), default)]
    pub class_name: Option<ClassName>,
    #[builder(setter(strip_option), default)]
    pub outcome: Option<TaskOutcome>,
    #[builder(setter(strip_option), default)]
    pub cacheable: Option<bool>,
    #[builder(setter(strip_option), default)]
    pub start_timestamp: Option<Timestamp>,
    #[builder(setter(strip_option), default)]
    pub finish_timestamp: Option<Timestamp>,
    #[builder(setter(strip_option), default)]
    pub cache_key: Option<CacheKey>,
    #[builder(setter(strip_option), default)]
    pub origin_execution_time: Option<Duration>,
    #[builder(setter(strip_option), default)]
    pub caching_disabled_reason: Option<String>,
    #[builder(setter(strip_option), default)]
    pub caching_disabled_explanation: Option<String>,
    #[builder(setter(strip_option), default)]
    pub up_to_date_messages: Option<Vec<String>>,
    #[builder(setter(strip_option), default)]
    pub origin_build_invocation_id: Option<String>,
}

#[derive(Debug, Clone, Builder)]
#[builder(setter(into))]
pub struct Test {
    pub id: TestId,
    pub scan_id: BuildScanId,
    pub class_name: TestClassName,
    #[builder(setter(strip_option), default)]
    pub method_name: Option<MethodName>,
    #[builder(setter(strip_option), default)]
    pub executor_name: Option<ExecutorName>,
    #[builder(setter(strip_option), default)]
    pub outcome: Option<TestOutcome>,
    #[builder(setter(strip_option), default)]
    pub duration_ms: Option<Duration>,
    #[builder(setter(strip_option), default)]
    pub failure_message: Option<String>,
    #[builder(setter(strip_option), default)]
    pub failure_stacktrace: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TestSummary {
    pub passed: TestCount,
    pub failed: TestCount,
    pub skipped: TestCount,
    pub total_duration_ms: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct CacheOperation {
    pub id: CacheOperationId,
    pub task_id: TaskId,
    pub operation_type: CacheOperationType,
    pub succeeded: bool,
    pub archive_size: Option<ArchiveSize>,
    pub cache_key: Option<String>,
    pub duration_ms: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct TaskDependencyGraph {
    pub nodes: Vec<TaskDependencyNode>,
    pub edges: Vec<TaskDependencyEdge>,
}

#[derive(Debug, Clone)]
pub struct TaskDependencyNode {
    pub id: TaskId,
}

#[derive(Debug, Clone)]
pub struct TaskDependencyEdge {
    pub source_id: TaskId,
    pub target_id: TaskId,
}
