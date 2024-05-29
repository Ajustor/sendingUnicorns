use std::fs;

use crate::config::home;

use crate::services::structs::{CollectionsConfig, Config};

fn get_collections_path() -> String {
    let base_path = home::get();
    return format!("{base_path}/collections");
}

fn get_collection_path(collection_name: &str) -> String {
    let base_path = get_collections_path();
    return format!("{base_path}/{collection_name}.json");
}

pub fn get_collections() -> Vec<CollectionsConfig> {
    let path = get_collections_path();
    let files = fs::read_dir(path).expect("Not a folder");
    let mut confs: Vec<CollectionsConfig> = Vec::new();

    for file in files.map(|file| file.unwrap().file_name()) {
        confs.push(CollectionsConfig {
            name: file.to_str().unwrap().to_string().replace(".json", ""),
            configs: Vec::new(),
        });
    }

    return confs;
}

pub fn write_collection(collection_name: &str, config: Config) {
    let path = get_collection_path(collection_name);
    let strigified_config = serde_json::to_string(&config).expect("Error while parsing json");
    let _ = fs::write(path, strigified_config);
}

pub fn delete_collection(collection_name: &str) {
    let path = get_collection_path(collection_name);

    fs::remove_file(path).expect("Error while deleting file");
}

pub fn read_collection(collection_name: &str) -> Config {
    let path = get_collection_path(collection_name);
    let collection_config_result = fs::read_to_string(path.clone());

    let collection_config = match collection_config_result {
        Ok(file) => file,
        Err(error) => panic!("Problem opening the file: {:?}", error),
    };

    return serde_json::from_str(&collection_config).expect("JSON was not well-formatted");
}
