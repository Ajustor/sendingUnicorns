use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;

#[derive(Debug, Deserialize, Serialize, Type, Clone)]
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
    pub value: DynamicValue,
}

#[derive(Debug, Deserialize, Serialize, Clone, Type)]
pub struct RequestOptions {
    pub body: BodyTypes,
    #[specta(type = Vec<(String, Options)>)]
    pub params: Option<Vec<(String, Options)>>,
    #[specta(type = Vec<(String, Options)>)]
    pub headers: Option<Vec<(String, Options)>>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
#[serde(untagged)]
pub enum DynamicValue {
    String(String),
    Number(i32),
    Boolean(bool),
    Null, // Représenter une valeur null
          // Ajoutez d'autres types si nécessaire
}

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub enum BodyTypesEnum {
    Json,
    FormData,
}

#[derive(Debug, Deserialize, Serialize, Clone, Type)]
pub struct BodyTypes {
    pub json: String,
    pub form_data: Vec<(String, Options)>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Type)]
pub struct RequestParams {
    pub body: Option<BodyTypes>,
    pub params: Option<Vec<(String, Value)>>,
    #[specta(type = Vec<[String; 2]>)]
    pub headers: Option<Vec<[String; 2]>>,
}
