use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Request {
    pub name: String,
    pub url: String,
    pub method: String,
    pub id: Option<Uuid>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CollectionConfig {
    pub name: String,
    pub requests: Vec<Request>,
}

// #[derive(Debug, Deserialize, Serialize)]
// pub struct RequestOptions {
//     body: Option<HashMap<any, any>>,
//     params: Option,
// }
