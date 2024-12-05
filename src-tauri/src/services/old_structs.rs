use serde::{Deserialize, Serialize};

use super::structs::{Environment, Options};

#[derive(Debug, Deserialize, Serialize)]
pub struct OldCollectionConfig {
    pub name: String,
    pub requests: Vec<OldRequest>,
    pub environments: Option<Vec<Environment>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OldRequest {
    pub name: String,
    pub url: String,
    pub method: String,
    pub id: Option<String>,
    pub pre_request_script: Option<String>,
    pub test: Option<String>,
    pub options: Option<OldRequestOptions>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OldRequestOptions {
    pub body: Option<BodyTypesConverter>,
    pub params: Option<Vec<(String, Options)>>,
    pub headers: Option<Vec<(String, Options)>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum BodyTypesConverter {
    FirstBodyType(Vec<(String, Options)>),
    V2OldBodyTypes(OldBodyTypes),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type", content = "value")]
pub enum OldBodyTypes {
    Json(String),
    FormData(Vec<(String, Options)>),
    Null,
}
