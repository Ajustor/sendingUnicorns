use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Request {
    pub name: String,
    pub url: String,
    pub method: String,
    pub id: Option<Uuid>,
    pub pre_request_script: Option<String>,
    pub test: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CollectionConfig {
    pub name: String,
    pub requests: Vec<Request>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestOptions {
    pub body: Option<Value>,
    pub params: Option<Value>,
    pub headers: Option<Value>,
}
