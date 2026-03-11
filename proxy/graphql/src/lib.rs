use std::sync::Arc;

use axum::Extension;
use juniper::{
    EmptyMutation, EmptySubscription, FieldError, FieldResult, ID, RootNode, graphql_object,
};
use juniper_axum::{extract::JuniperRequest, response::JuniperResponse};

use domain;
use relay::{Cursor, RelayId, validate_pagination};
use service::ProxyService;

pub struct Context {
    pub service: Arc<ProxyService>,
}

impl juniper::Context for Context {}

// ---------------------------------------------------------------------------
// Payload type
// ---------------------------------------------------------------------------

pub struct Payload {
    pub payload: domain::Payload,
}

#[graphql_object(context = Context)]
impl Payload {
    fn id(&self) -> ID {
        RelayId::encode("Payload", &self.payload.id.0.to_string())
    }

    fn request_id(&self) -> &str {
        &self.payload.request_id
    }

    fn timestamp(&self) -> &str {
        &self.payload.timestamp
    }

    fn request(&self) -> RequestData {
        RequestData {
            method: self.payload.request.method.clone(),
            uri: self.payload.request.uri.clone(),
            headers: self
                .payload
                .request
                .headers
                .iter()
                .map(|h| Header {
                    name: h.name.clone(),
                    value: h.value.clone(),
                })
                .collect(),
            body: self.payload.request.body.clone(),
        }
    }

    fn response(&self) -> ResponseData {
        ResponseData {
            status: self.payload.response.status,
            headers: self.payload.response.headers.as_ref().map(|hs| {
                hs.iter()
                    .map(|h| Header {
                        name: h.name.clone(),
                        value: h.value.clone(),
                    })
                    .collect()
            }),
            body: self.payload.response.body.clone(),
            error: self.payload.response.error.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Nested value types
// ---------------------------------------------------------------------------

pub struct Header {
    pub name: String,
    pub value: String,
}

#[graphql_object(context = Context)]
impl Header {
    fn name(&self) -> &str {
        &self.name
    }

    fn value(&self) -> &str {
        &self.value
    }
}

pub struct RequestData {
    pub method: String,
    pub uri: String,
    pub headers: Vec<Header>,
    pub body: Option<String>,
}

#[graphql_object(context = Context)]
impl RequestData {
    fn method(&self) -> &str {
        &self.method
    }

    fn uri(&self) -> &str {
        &self.uri
    }

    fn headers(&self) -> &Vec<Header> {
        &self.headers
    }

    fn body(&self) -> Option<&str> {
        self.body.as_deref()
    }
}

pub struct ResponseData {
    pub status: Option<i32>,
    pub headers: Option<Vec<Header>>,
    pub body: Option<String>,
    pub error: Option<String>,
}

#[graphql_object(context = Context)]
impl ResponseData {
    fn status(&self) -> Option<i32> {
        self.status
    }

    fn headers(&self) -> &Option<Vec<Header>> {
        &self.headers
    }

    fn body(&self) -> Option<&str> {
        self.body.as_deref()
    }

    fn error(&self) -> Option<&str> {
        self.error.as_deref()
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
// PayloadConnection / PayloadEdge
// ---------------------------------------------------------------------------

pub struct PayloadEdge {
    pub cursor: String,
    pub node: Payload,
}

#[graphql_object(context = Context)]
impl PayloadEdge {
    fn cursor(&self) -> &str {
        &self.cursor
    }

    fn node(&self) -> &Payload {
        &self.node
    }
}

pub struct PayloadConnection {
    pub edges: Vec<PayloadEdge>,
    pub page_info: PageInfo,
}

#[graphql_object(context = Context)]
impl PayloadConnection {
    fn edges(&self) -> &Vec<PayloadEdge> {
        &self.edges
    }

    fn page_info(&self) -> &PageInfo {
        &self.page_info
    }

    async fn total_count(&self, context: &Context) -> FieldResult<i32> {
        let count = context
            .service
            .count_payloads()
            .await
            .map_err(|e| FieldError::from(e.to_string()))? as i32;
        Ok(count)
    }
}

// ---------------------------------------------------------------------------
// QueryRoot
// ---------------------------------------------------------------------------

pub struct QueryRoot;

#[graphql_object(context = Context)]
impl QueryRoot {
    async fn node(context: &Context, id: ID) -> FieldResult<Option<Payload>> {
        let relay_id = RelayId::decode(&id).map_err(|e| FieldError::from(e.to_string()))?;
        match relay_id.type_name.as_str() {
            "Payload" => {
                let payload = context
                    .service
                    .get_payload(&relay_id.raw_id)
                    .await
                    .map_err(|e| FieldError::from(e.to_string()))?;
                Ok(payload.map(|p| Payload { payload: p }))
            }
            _ => Ok(None),
        }
    }

    async fn payload(context: &Context, id: ID) -> FieldResult<Option<Payload>> {
        // Support both raw UUID and Relay global ID
        let raw_id = if let Ok(relay_id) = RelayId::decode(&id) {
            relay_id.raw_id
        } else {
            id.to_string()
        };

        let payload = context
            .service
            .get_payload(&raw_id)
            .await
            .map_err(|e| FieldError::from(e.to_string()))?;

        Ok(payload.map(|p| Payload { payload: p }))
    }

    async fn payloads(
        context: &Context,
        first: Option<i32>,
        after: Option<String>,
    ) -> FieldResult<PayloadConnection> {
        let limit = validate_pagination(first).map_err(|e| FieldError::from(e.to_string()))?;
        let fetch_limit = (limit + 1) as i64;

        let after_id = after
            .as_deref()
            .map(Cursor::decode)
            .transpose()
            .map_err(|e| FieldError::from(e.to_string()))?
            .map(|c| c.value);

        let mut payloads = context
            .service
            .list_payloads(fetch_limit, after_id.as_deref())
            .await
            .map_err(|e| FieldError::from(e.to_string()))?;

        let has_next_page = payloads.len() > limit as usize;
        if has_next_page {
            payloads.pop();
        }

        let end_cursor = payloads
            .last()
            .map(|p| Cursor::new(p.id.0.to_string()).encode());

        let edges: Vec<PayloadEdge> = payloads
            .into_iter()
            .map(|p| {
                let cursor = Cursor::new(p.id.0.to_string()).encode();
                PayloadEdge {
                    cursor,
                    node: Payload { payload: p },
                }
            })
            .collect();

        Ok(PayloadConnection {
            edges,
            page_info: PageInfo {
                has_next_page,
                end_cursor,
            },
        })
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
