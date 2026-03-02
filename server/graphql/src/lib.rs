use std::sync::Arc;

use axum::Extension;
use juniper::{
    EmptyMutation, EmptySubscription, FieldError, FieldResult, GraphQLInterface, ID, RootNode,
    graphql_object,
};
use juniper_axum::{extract::JuniperRequest, response::JuniperResponse};
use relay::{Cursor, RelayId, validate_pagination};
use sqlx::SqlitePool;

pub struct Context {
    pub pool: Arc<SqlitePool>,
}

impl juniper::Context for Context {}

// ---------------------------------------------------------------------------
// Node interface (Relay Global Object Identification)
// ---------------------------------------------------------------------------

#[derive(GraphQLInterface)]
#[graphql(for = [BuildScan, Task], context = Context)]
pub struct Node {
    pub id: ID,
}

// ---------------------------------------------------------------------------
// BuildScan type
// ---------------------------------------------------------------------------

pub struct BuildScan {
    pub row: db::BuildScanRow,
}

#[graphql_object(context = Context, impl = NodeValue)]
impl BuildScan {
    fn id(&self) -> ID {
        RelayId::encode("BuildScan", &self.row.id)
    }

    fn scan_id(&self) -> &str {
        &self.row.id
    }

    fn build_tool_type(&self) -> &str {
        &self.row.build_tool_type
    }

    fn build_tool_version(&self) -> &str {
        &self.row.build_tool_version
    }

    fn plugin_version(&self) -> &str {
        &self.row.plugin_version
    }

    fn started_at(&self) -> Option<&str> {
        self.row.started_at.as_deref()
    }

    fn finished_at(&self) -> Option<&str> {
        self.row.finished_at.as_deref()
    }

    fn outcome(&self) -> Option<&str> {
        self.row.outcome.as_deref()
    }

    fn hostname(&self) -> Option<&str> {
        self.row.hostname.as_deref()
    }

    fn os_name(&self) -> Option<&str> {
        self.row.os_name.as_deref()
    }

    fn os_version(&self) -> Option<&str> {
        self.row.os_version.as_deref()
    }

    fn jvm_vendor(&self) -> Option<&str> {
        self.row.jvm_vendor.as_deref()
    }

    fn jvm_version(&self) -> Option<&str> {
        self.row.jvm_version.as_deref()
    }

    fn created_at(&self) -> &str {
        &self.row.created_at
    }

    fn requested_tasks(&self) -> Vec<String> {
        self.row
            .requested_tasks
            .as_deref()
            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
            .unwrap_or_default()
    }

    async fn task_count(&self, context: &Context) -> FieldResult<i32> {
        let count = db::count_tasks(&context.pool, &self.row.id)
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
        let limit = validate_pagination(first).map_err(FieldError::from)?;
        let after_id = after
            .as_deref()
            .map(Cursor::decode)
            .transpose()
            .map_err(FieldError::from)?
            .map(|c| c.value);

        let mut rows = db::list_tasks(
            &context.pool,
            &self.row.id,
            (limit + 1) as i64,
            after_id.as_deref(),
        )
        .await
        .map_err(|e| FieldError::from(e.to_string()))?;

        let has_next_page = rows.len() > limit as usize;
        if has_next_page {
            rows.pop();
        }

        let end_cursor = rows.last().map(|r| Cursor::new(r.id.clone()).encode());

        let edges: Vec<TaskEdge> = rows
            .into_iter()
            .map(|r| {
                let cursor = Cursor::new(r.id.clone()).encode();
                TaskEdge {
                    cursor,
                    node: Task { row: r },
                }
            })
            .collect();

        let total_count = db::count_tasks(&context.pool, &self.row.id)
            .await
            .map_err(|e| FieldError::from(e.to_string()))? as i32;

        Ok(TaskConnection {
            edges,
            page_info: PageInfo {
                has_next_page,
                end_cursor,
            },
            total_count,
        })
    }
}

// ---------------------------------------------------------------------------
// Task type
// ---------------------------------------------------------------------------

pub struct Task {
    pub row: db::TaskRow,
}

#[graphql_object(context = Context, impl = NodeValue)]
impl Task {
    fn id(&self) -> ID {
        RelayId::encode("Task", &self.row.id)
    }

    fn task_id(&self) -> &str {
        &self.row.id
    }

    fn scan_id(&self) -> &str {
        &self.row.scan_id
    }

    fn task_path(&self) -> &str {
        &self.row.task_path
    }

    fn class_name(&self) -> Option<&str> {
        self.row.class_name.as_deref()
    }

    fn outcome(&self) -> Option<&str> {
        self.row.outcome.as_deref()
    }

    fn cacheable(&self) -> Option<bool> {
        self.row.cacheable
    }

    fn start_timestamp(&self) -> Option<f64> {
        self.row.start_timestamp.map(|v| v as f64)
    }

    fn finish_timestamp(&self) -> Option<f64> {
        self.row.finish_timestamp.map(|v| v as f64)
    }

    fn duration_ms(&self) -> Option<f64> {
        match (self.row.start_timestamp, self.row.finish_timestamp) {
            (Some(start), Some(finish)) => Some((finish - start) as f64),
            _ => None,
        }
    }

    fn cache_key(&self) -> Option<&str> {
        self.row.cache_key.as_deref()
    }

    fn origin_execution_time(&self) -> Option<f64> {
        self.row.origin_execution_time.map(|v| v as f64)
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
    pub total_count: i32,
}

#[graphql_object(context = Context)]
impl BuildScanConnection {
    fn edges(&self) -> &Vec<BuildScanEdge> {
        &self.edges
    }

    fn page_info(&self) -> &PageInfo {
        &self.page_info
    }

    fn total_count(&self) -> i32 {
        self.total_count
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
    pub edges: Vec<TaskEdge>,
    pub page_info: PageInfo,
    pub total_count: i32,
}

#[graphql_object(context = Context)]
impl TaskConnection {
    fn edges(&self) -> &Vec<TaskEdge> {
        &self.edges
    }

    fn page_info(&self) -> &PageInfo {
        &self.page_info
    }

    fn total_count(&self) -> i32 {
        self.total_count
    }
}

// ---------------------------------------------------------------------------
// QueryRoot
// ---------------------------------------------------------------------------

fn encode_build_scan_cursor(row: &db::BuildScanRow) -> String {
    let value = serde_json::json!({"created_at": &row.created_at, "id": &row.id}).to_string();
    Cursor::new(value).encode()
}

pub struct QueryRoot;

#[graphql_object(context = Context)]
impl QueryRoot {
    async fn node(context: &Context, id: ID) -> FieldResult<Option<NodeValue>> {
        let relay_id = RelayId::decode(&id).map_err(FieldError::from)?;
        match relay_id.type_name.as_str() {
            "BuildScan" => {
                let row = db::get_build_scan(&context.pool, &relay_id.raw_id)
                    .await
                    .map_err(|e| FieldError::from(e.to_string()))?;
                Ok(row.map(|r| NodeValue::BuildScan(BuildScan { row: r })))
            }
            "Task" => {
                let row = db::get_task(&context.pool, &relay_id.raw_id)
                    .await
                    .map_err(|e| FieldError::from(e.to_string()))?;
                Ok(row.map(|r| NodeValue::Task(Task { row: r })))
            }
            _ => Ok(None),
        }
    }

    async fn build_scans(
        context: &Context,
        first: Option<i32>,
        after: Option<String>,
    ) -> FieldResult<BuildScanConnection> {
        let limit = validate_pagination(first).map_err(FieldError::from)?;
        let cursor = after
            .as_deref()
            .map(Cursor::decode)
            .transpose()
            .map_err(FieldError::from)?;
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

        let mut rows = db::list_build_scans(
            &context.pool,
            (limit + 1) as i64,
            after_created_at.as_deref(),
            after_id.as_deref(),
        )
        .await
        .map_err(|e| FieldError::from(e.to_string()))?;

        let has_next_page = rows.len() > limit as usize;
        if has_next_page {
            rows.pop();
        }

        let end_cursor = rows.last().map(|r| encode_build_scan_cursor(r));

        let edges: Vec<BuildScanEdge> = rows
            .into_iter()
            .map(|r| {
                let cursor = encode_build_scan_cursor(&r);
                BuildScanEdge {
                    cursor,
                    node: BuildScan { row: r },
                }
            })
            .collect();

        let total_count = db::count_build_scans(&context.pool)
            .await
            .map_err(|e| FieldError::from(e.to_string()))? as i32;

        Ok(BuildScanConnection {
            edges,
            page_info: PageInfo {
                has_next_page,
                end_cursor,
            },
            total_count,
        })
    }

    async fn build_scan(context: &Context, id: ID) -> FieldResult<Option<BuildScan>> {
        // Support both raw UUID and Relay global ID
        let raw_id = if let Ok(relay_id) = RelayId::decode(&id) {
            relay_id.raw_id
        } else {
            id.to_string()
        };

        let row = db::get_build_scan(&context.pool, &raw_id)
            .await
            .map_err(|e| FieldError::from(e.to_string()))?;

        Ok(row.map(|r| BuildScan { row: r }))
    }
}

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

pub type Schema = RootNode<'static, QueryRoot, EmptyMutation<Context>, EmptySubscription<Context>>;

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
