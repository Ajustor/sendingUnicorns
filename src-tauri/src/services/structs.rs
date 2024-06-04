use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::{Any, Type};

#[derive(Debug, Deserialize, Serialize, Type)]
pub struct CollectionConfig {
    pub name: String,
    pub requests: Vec<Request>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Type)]
pub struct Request {
    pub name: String,
    pub url: String,
    pub method: String,
    #[specta(type = String)]
    pub id: Option<String>,
    pub pre_request_script: Option<String>,
    pub test: Option<String>,
    #[specta(type = RequestOptions)]
    pub options: Option<RequestOptions>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Type)]
pub struct RequestOptions {
    #[specta(type = HashMap<String, Any>)]
    pub body: Option<HashMap<String, Value>>,
    #[specta(type = HashMap<String, Any>)]
    pub params: Option<HashMap<String, Value>>,
    #[specta(type = Vec<[String; 2]>)]
    pub headers: Option<Vec<[String; 2]>>,
}
