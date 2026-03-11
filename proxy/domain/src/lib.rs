use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PayloadId(pub Uuid);

#[derive(Debug, Clone)]
pub struct Header {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct RequestData {
    pub method: String,
    pub uri: String,
    pub headers: Vec<Header>,
    pub body: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResponseData {
    pub status: Option<i32>,
    pub headers: Option<Vec<Header>>,
    pub body: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Payload {
    pub id: PayloadId,
    pub request_id: String,
    pub timestamp: String,
    pub request: RequestData,
    pub response: ResponseData,
    pub created_at: String,
}
