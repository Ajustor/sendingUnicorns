use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CollectionsConfig {
    pub name: String,
    pub configs: Vec<Config>,
}
