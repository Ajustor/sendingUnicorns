use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize)]
pub struct Request {
    pub name: String,
    pub url: String,
    pub method: String
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CollectionConfig {
    pub name: String,
    pub requests: Vec<Request>
}
