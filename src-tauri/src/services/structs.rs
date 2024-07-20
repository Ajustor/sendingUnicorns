use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::{Any, Type};

#[derive(Debug, Deserialize, Serialize, Type)]
pub struct CollectionConfig {
    pub name: String,
    pub requests: Vec<Request>,
    pub environments: Option<Vec<Environment>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Type)]
pub struct Environment {
    pub name: String,
    #[specta(type = String)]
    pub id: Option<String>,
    #[specta(type = Vec<[String; 2]>)]
    pub vars: Option<Vec<[String; 2]>>,
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
pub struct Options {
    pub is_active: bool,
    #[specta(type = Any)]
    pub value: Value,
}

#[derive(Debug, Deserialize, Serialize, Clone, Type)]
pub struct RequestOptions {
    #[specta(type = Vec<(String, Options)>)]
    pub body: Option<Vec<(String, Options)>>,
    #[specta(type = Vec<(String, Options)>)]
    pub params: Option<Vec<(String, Options)>>,
    #[specta(type = Vec<(String, Options)>)]
    pub headers: Option<Vec<(String, Options)>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Type)]
pub struct RequestParams {
    #[specta(type = Vec<[Any; 2]>)]
    pub body: Option<Vec<(String, Value)>>,
    #[specta(type = Vec<[Any; 2]>)]
    pub params: Option<Vec<(String, Value)>>,
    #[specta(type = Vec<[String; 2]>)]
    pub headers: Option<Vec<[String; 2]>>,
}
