use std::sync::Arc;

use axum::Extension;
use juniper::{
    EmptyMutation, EmptySubscription, FieldError, FieldResult, GraphQLInterface, ID, RootNode,
    graphql_object,
};
use juniper_axum::{extract::JuniperRequest, response::JuniperResponse};
use relay::{Cursor, RelayId, validate_pagination};
use service::BuildScanService;

pub struct Context {
    pub service: Arc<BuildScanService>,
}

impl juniper::Context for Context {}

// ---------------------------------------------------------------------------
// Node interface (Relay Global Object Identification)
// ---------------------------------------------------------------------------

#[derive(GraphQLInterface)]
#[graphql(for = [BuildScan, Task, Test], context = Context)]
pub struct Node {
    pub id: ID,
}

// ---------------------------------------------------------------------------
// BuildScan type
// ---------------------------------------------------------------------------

pub struct BuildScan {
    pub scan: domain::BuildScan,
}

#[graphql_object(context = Context, impl = NodeValue)]
impl BuildScan {
    fn id(&self) -> ID {
        RelayId::encode("BuildScan", &self.scan.id.0.to_string())
    }

    fn scan_id(&self) -> String {
        self.scan.id.0.to_string()
    }

    fn build_tool_type(&self) -> &str {
        &self.scan.build_tool_type.0
    }

    fn build_tool_version(&self) -> &str {
        &self.scan.build_tool_version.0
    }

    fn plugin_version(&self) -> &str {
        &self.scan.plugin_version.0
    }

    fn started_at(&self) -> Option<String> {
        self.scan.started_at.map(|dt| dt.to_rfc3339())
    }

    fn finished_at(&self) -> Option<String> {
        self.scan.finished_at.map(|dt| dt.to_rfc3339())
    }

    fn outcome(&self) -> Option<String> {
        self.scan.outcome.map(|o| o.to_string())
    }

    fn hostname(&self) -> Option<&str> {
        self.scan.hostname.as_ref().map(|h| h.0.as_str())
    }

    fn os_name(&self) -> Option<&str> {
        self.scan.os_name.as_ref().map(|n| n.0.as_str())
    }

    fn os_version(&self) -> Option<&str> {
        self.scan.os_version.as_ref().map(|v| v.0.as_str())
    }

    fn jvm_vendor(&self) -> Option<&str> {
        self.scan.jvm_vendor.as_ref().map(|v| v.0.as_str())
    }

    fn jvm_version(&self) -> Option<&str> {
        self.scan.jvm_version.as_ref().map(|v| v.0.as_str())
    }

    fn created_at(&self) -> String {
        self.scan.created_at.to_rfc3339()
    }

    fn requested_tasks(&self) -> Vec<String> {
        self.scan
            .requested_tasks
            .as_ref()
            .map(|tasks| tasks.iter().map(|t| t.0.clone()).collect())
            .unwrap_or_default()
    }

    async fn task_count(&self, context: &Context) -> FieldResult<i32> {
        let count = context
            .service
            .count_tasks(&self.scan.id.0.to_string())
            .await
            .map_err(|e| FieldError::from(e.to_string()))? as i32;
        Ok(count)
    }

    async fn tasks(
        &self,
        context: &Context,
        first: Option<i32>,
        after: Option<String>,
    ) -> FieldResult<TaskConnection> {
        let limit = validate_pagination(first).map_err(|e| FieldError::from(e.to_string()))?;
        let after_id = after
            .as_deref()
            .map(Cursor::decode)
            .transpose()
            .map_err(|e| FieldError::from(e.to_string()))?
            .map(|c| c.value);

        let scan_id_str = self.scan.id.0.to_string();
        let mut tasks = context
            .service
            .list_tasks(&scan_id_str, (limit + 1) as i64, after_id.as_deref())
            .await
            .map_err(|e| FieldError::from(e.to_string()))?;

        let has_next_page = tasks.len() > limit as usize;
        if has_next_page {
            tasks.pop();
        }

        let end_cursor = tasks
            .last()
            .map(|t| Cursor::new(t.id.0.to_string()).encode());

        let edges: Vec<TaskEdge> = tasks
            .into_iter()
            .map(|t| {
                let cursor = Cursor::new(t.id.0.to_string()).encode();
                TaskEdge {
                    cursor,
                    node: Task { task: t },
                }
            })
            .collect();

        Ok(TaskConnection {
            scan_id: scan_id_str,
            edges,
            page_info: PageInfo {
                has_next_page,
                end_cursor,
            },
        })
    }

    async fn test_count(&self, context: &Context) -> FieldResult<i32> {
        let count = context
            .service
            .count_tests(&self.scan.id.0.to_string())
            .await
            .map_err(|e| FieldError::from(e.to_string()))? as i32;
        Ok(count)
    }

    async fn test_summary(&self, context: &Context) -> FieldResult<TestSummary> {
        let summary = context
            .service
            .test_summary(&self.scan.id.0.to_string())
            .await
            .map_err(|e| FieldError::from(e.to_string()))?;
        Ok(TestSummary { summary })
    }

    async fn tests(
        &self,
        context: &Context,
        first: Option<i32>,
        after: Option<String>,
    ) -> FieldResult<TestCaseConnection> {
        let limit = validate_pagination(first).map_err(|e| FieldError::from(e.to_string()))?;
        let after_id = after
            .as_deref()
            .map(Cursor::decode)
            .transpose()
            .map_err(|e| FieldError::from(e.to_string()))?
            .map(|c| c.value);

        let scan_id_str = self.scan.id.0.to_string();
        let mut tests = context
            .service
            .list_tests(&scan_id_str, (limit + 1) as i64, after_id.as_deref())
            .await
            .map_err(|e| FieldError::from(e.to_string()))?;

        let has_next_page = tests.len() > limit as usize;
        if has_next_page {
            tests.pop();
        }

        let end_cursor = tests
            .last()
            .map(|t| Cursor::new(t.id.0.to_string()).encode());

        let edges: Vec<TestCaseEdge> = tests
            .into_iter()
            .map(|t| {
                let cursor = Cursor::new(t.id.0.to_string()).encode();
                TestCaseEdge {
                    cursor,
                    node: Test { test: t },
                }
            })
            .collect();

        Ok(TestCaseConnection {
            scan_id: scan_id_str,
            edges,
            page_info: PageInfo {
                has_next_page,
                end_cursor,
            },
        })
    }
}

// ---------------------------------------------------------------------------
// Task type
// ---------------------------------------------------------------------------

pub struct Task {
    pub task: domain::Task,
}

#[graphql_object(context = Context, impl = NodeValue)]
impl Task {
    fn id(&self) -> ID {
        RelayId::encode("Task", &self.task.id.0.to_string())
    }

    fn task_id(&self) -> String {
        self.task.id.0.to_string()
    }

    fn scan_id(&self) -> String {
        self.task.scan_id.0.to_string()
    }

    fn task_path(&self) -> &str {
        &self.task.task_path.0
    }

    fn class_name(&self) -> Option<&str> {
        self.task.class_name.as_ref().map(|c| c.0.as_str())
    }

    fn outcome(&self) -> Option<String> {
        self.task.outcome.map(|o| o.to_string())
    }

    fn cacheable(&self) -> Option<bool> {
        self.task.cacheable
    }

    fn start_timestamp(&self) -> Option<f64> {
        self.task.start_timestamp.map(|t| t.0 as f64)
    }

    fn finish_timestamp(&self) -> Option<f64> {
        self.task.finish_timestamp.map(|t| t.0 as f64)
    }

    fn duration_ms(&self) -> Option<f64> {
        match (self.task.start_timestamp, self.task.finish_timestamp) {
            (Some(start), Some(finish)) => Some((finish.0 - start.0) as f64),
            _ => None,
        }
    }

    fn cache_key(&self) -> Option<&str> {
        self.task.cache_key.as_ref().map(|k| k.0.as_str())
    }

    fn origin_execution_time(&self) -> Option<f64> {
        self.task.origin_execution_time.map(|d| d.0 as f64)
    }

    fn caching_disabled_reason(&self) -> Option<&str> {
        self.task.caching_disabled_reason.as_deref()
    }

    fn caching_disabled_explanation(&self) -> Option<&str> {
        self.task.caching_disabled_explanation.as_deref()
    }

    fn up_to_date_messages(&self) -> Option<&Vec<String>> {
        self.task.up_to_date_messages.as_ref()
    }

    fn origin_build_invocation_id(&self) -> Option<&str> {
        self.task.origin_build_invocation_id.as_deref()
    }

    async fn cache_operations(&self, context: &Context) -> FieldResult<Vec<CacheOperation>> {
        let task_id = self.task.id.0.to_string();
        let ops = context
            .service
            .list_cache_operations(&task_id)
            .await
            .map_err(|e| FieldError::from(e.to_string()))?;
        Ok(ops.into_iter().map(|op| CacheOperation { op }).collect())
    }
}

// ---------------------------------------------------------------------------
// CacheOperation type
// ---------------------------------------------------------------------------

pub struct CacheOperation {
    pub op: domain::CacheOperation,
}

#[graphql_object(context = Context)]
impl CacheOperation {
    fn id(&self) -> String {
        self.op.id.0.to_string()
    }

    fn operation_type(&self) -> String {
        self.op.operation_type.to_string()
    }

    fn succeeded(&self) -> bool {
        self.op.succeeded
    }

    fn archive_size(&self) -> Option<f64> {
        self.op.archive_size.map(|s| s.0 as f64)
    }

    fn cache_key(&self) -> Option<&str> {
        self.op.cache_key.as_deref()
    }

    fn duration_ms(&self) -> Option<f64> {
        self.op.duration_ms.map(|d| d.0 as f64)
    }
}

// ---------------------------------------------------------------------------
// Test type
// ---------------------------------------------------------------------------

pub struct Test {
    pub test: domain::Test,
}

#[graphql_object(context = Context, impl = NodeValue)]
impl Test {
    fn id(&self) -> ID {
        RelayId::encode("Test", &self.test.id.0.to_string())
    }

    fn test_id(&self) -> String {
        self.test.id.0.to_string()
    }

    fn scan_id(&self) -> String {
        self.test.scan_id.0.to_string()
    }

    fn class_name(&self) -> &str {
        &self.test.class_name.0
    }

    fn method_name(&self) -> Option<&str> {
        self.test.method_name.as_ref().map(|m| m.0.as_str())
    }

    fn executor_name(&self) -> Option<&str> {
        self.test.executor_name.as_ref().map(|e| e.0.as_str())
    }

    fn outcome(&self) -> Option<String> {
        self.test.outcome.map(|o| o.to_string())
    }

    fn duration_ms(&self) -> FieldResult<Option<i32>> {
        self.test
            .duration_ms
            .map(|d| {
                i32::try_from(d.0)
                    .map_err(|_| FieldError::from("duration_ms exceeds GraphQL Int range"))
            })
            .transpose()
    }

    fn failure_message(&self) -> Option<&str> {
        self.test.failure_message.as_deref()
    }

    fn failure_stacktrace(&self) -> Option<&str> {
        self.test.failure_stacktrace.as_deref()
    }
}

// ---------------------------------------------------------------------------
// TestSummary type
// ---------------------------------------------------------------------------

pub struct TestSummary {
    pub summary: domain::TestSummary,
}

#[graphql_object(context = Context)]
impl TestSummary {
    fn passed(&self) -> i32 {
        self.summary.passed.0 as i32
    }

    fn failed(&self) -> i32 {
        self.summary.failed.0 as i32
    }

    fn skipped(&self) -> i32 {
        self.summary.skipped.0 as i32
    }

    fn total_duration_ms(&self) -> FieldResult<Option<i32>> {
        self.summary
            .total_duration_ms
            .map(|d| {
                i32::try_from(d.0)
                    .map_err(|_| FieldError::from("total_duration_ms exceeds GraphQL Int range"))
            })
            .transpose()
    }
}

// ---------------------------------------------------------------------------
// PageInfo
// ---------------------------------------------------------------------------

pub struct PageInfo {
    pub has_next_page: bool,
    pub end_cursor: Option<String>,
}

#[graphql_object(context = Context)]
impl PageInfo {
    fn has_next_page(&self) -> bool {
        self.has_next_page
    }

    fn end_cursor(&self) -> Option<&str> {
        self.end_cursor.as_deref()
    }
}

// ---------------------------------------------------------------------------
// BuildScanConnection / BuildScanEdge
// ---------------------------------------------------------------------------

pub struct BuildScanEdge {
    pub cursor: String,
    pub node: BuildScan,
}

#[graphql_object(context = Context)]
impl BuildScanEdge {
    fn cursor(&self) -> &str {
        &self.cursor
    }

    fn node(&self) -> &BuildScan {
        &self.node
    }
}

pub struct BuildScanConnection {
    pub edges: Vec<BuildScanEdge>,
    pub page_info: PageInfo,
}

#[graphql_object(context = Context)]
impl BuildScanConnection {
    fn edges(&self) -> &Vec<BuildScanEdge> {
        &self.edges
    }

    fn page_info(&self) -> &PageInfo {
        &self.page_info
    }

    async fn total_count(&self, context: &Context) -> FieldResult<i32> {
        let count = context
            .service
            .count_build_scans()
            .await
            .map_err(|e| FieldError::from(e.to_string()))? as i32;
        Ok(count)
    }
}

// ---------------------------------------------------------------------------
// TaskConnection / TaskEdge
// ---------------------------------------------------------------------------

pub struct TaskEdge {
    pub cursor: String,
    pub node: Task,
}

#[graphql_object(context = Context)]
impl TaskEdge {
    fn cursor(&self) -> &str {
        &self.cursor
    }

    fn node(&self) -> &Task {
        &self.node
    }
}

pub struct TaskConnection {
    pub scan_id: String,
    pub edges: Vec<TaskEdge>,
    pub page_info: PageInfo,
}

#[graphql_object(context = Context)]
impl TaskConnection {
    fn edges(&self) -> &Vec<TaskEdge> {
        &self.edges
    }

    fn page_info(&self) -> &PageInfo {
        &self.page_info
    }

    async fn total_count(&self, context: &Context) -> FieldResult<i32> {
        let count = context
            .service
            .count_tasks(&self.scan_id)
            .await
            .map_err(|e| FieldError::from(e.to_string()))? as i32;
        Ok(count)
    }
}

// ---------------------------------------------------------------------------
// TestCaseConnection / TestCaseEdge
// ---------------------------------------------------------------------------

pub struct TestCaseEdge {
    pub cursor: String,
    pub node: Test,
}

#[graphql_object(context = Context)]
impl TestCaseEdge {
    fn cursor(&self) -> &str {
        &self.cursor
    }

    fn node(&self) -> &Test {
        &self.node
    }
}

pub struct TestCaseConnection {
    pub scan_id: String,
    pub edges: Vec<TestCaseEdge>,
    pub page_info: PageInfo,
}

#[graphql_object(context = Context)]
impl TestCaseConnection {
    fn edges(&self) -> &Vec<TestCaseEdge> {
        &self.edges
    }

    fn page_info(&self) -> &PageInfo {
        &self.page_info
    }

    async fn total_count(&self, context: &Context) -> FieldResult<i32> {
        let count = context
            .service
            .count_tests(&self.scan_id)
            .await
            .map_err(|e| FieldError::from(e.to_string()))? as i32;
        Ok(count)
    }
}

// ---------------------------------------------------------------------------
// QueryRoot
// ---------------------------------------------------------------------------

fn encode_build_scan_cursor(scan: &domain::BuildScan) -> String {
    let value = serde_json::json!({
        "created_at": scan.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        "id": scan.id.0.to_string()
    })
    .to_string();
    Cursor::new(value).encode()
}

pub struct QueryRoot;

#[graphql_object(context = Context)]
impl QueryRoot {
    async fn node(context: &Context, id: ID) -> FieldResult<Option<NodeValue>> {
        let relay_id = RelayId::decode(&id).map_err(|e| FieldError::from(e.to_string()))?;
        match relay_id.type_name.as_str() {
            "BuildScan" => {
                let scan = context
                    .service
                    .get_build_scan(&relay_id.raw_id)
                    .await
                    .map_err(|e| FieldError::from(e.to_string()))?;
                Ok(scan.map(|s| NodeValue::BuildScan(BuildScan { scan: s })))
            }
            "Task" => {
                let task = context
                    .service
                    .get_task(&relay_id.raw_id)
                    .await
                    .map_err(|e| FieldError::from(e.to_string()))?;
                Ok(task.map(|t| NodeValue::Task(Task { task: t })))
            }
            "Test" => {
                let test = context
                    .service
                    .get_test(&relay_id.raw_id)
                    .await
                    .map_err(|e| FieldError::from(e.to_string()))?;
                Ok(test.map(|t| NodeValue::Test(Test { test: t })))
            }
            _ => Ok(None),
        }
    }

    async fn build_scans(
        context: &Context,
        first: Option<i32>,
        after: Option<String>,
    ) -> FieldResult<BuildScanConnection> {
        let limit = validate_pagination(first).map_err(|e| FieldError::from(e.to_string()))?;
        let cursor = after
            .as_deref()
            .map(Cursor::decode)
            .transpose()
            .map_err(|e| FieldError::from(e.to_string()))?;
        let (after_created_at, after_id) = if let Some(c) = cursor {
            let v: serde_json::Value = serde_json::from_str(&c.value)
                .map_err(|_| FieldError::from("Invalid cursor format".to_string()))?;
            let created_at = v["created_at"]
                .as_str()
                .ok_or_else(|| FieldError::from("Missing created_at in cursor".to_string()))?
                .to_string();
            let id = v["id"]
                .as_str()
                .ok_or_else(|| FieldError::from("Missing id in cursor".to_string()))?
                .to_string();
            (Some(created_at), Some(id))
        } else {
            (None, None)
        };

        let mut scans = context
            .service
            .list_build_scans(
                (limit + 1) as i64,
                after_created_at.as_deref(),
                after_id.as_deref(),
            )
            .await
            .map_err(|e| FieldError::from(e.to_string()))?;

        let has_next_page = scans.len() > limit as usize;
        if has_next_page {
            scans.pop();
        }

        let end_cursor = scans.last().map(encode_build_scan_cursor);

        let edges: Vec<BuildScanEdge> = scans
            .into_iter()
            .map(|s| {
                let cursor = encode_build_scan_cursor(&s);
                BuildScanEdge {
                    cursor,
                    node: BuildScan { scan: s },
                }
            })
            .collect();

        Ok(BuildScanConnection {
            edges,
            page_info: PageInfo {
                has_next_page,
                end_cursor,
            },
        })
    }

    async fn build_scan(context: &Context, id: ID) -> FieldResult<Option<BuildScan>> {
        // Support both raw UUID and Relay global ID
        let raw_id = if let Ok(relay_id) = RelayId::decode(&id) {
            relay_id.raw_id
        } else {
            id.to_string()
        };

        let scan = context
            .service
            .get_build_scan(&raw_id)
            .await
            .map_err(|e| FieldError::from(e.to_string()))?;

        Ok(scan.map(|s| BuildScan { scan: s }))
    }
}

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

pub type Schema = RootNode<QueryRoot, EmptyMutation<Context>, EmptySubscription<Context>>;

pub fn create_schema() -> Schema {
    Schema::new(QueryRoot, EmptyMutation::new(), EmptySubscription::new())
}

pub async fn graphql_handler(
    Extension(schema): Extension<Arc<Schema>>,
    Extension(context): Extension<Arc<Context>>,
    JuniperRequest(request): JuniperRequest,
) -> JuniperResponse {
    JuniperResponse(request.execute(&*schema, &*context).await)
}
