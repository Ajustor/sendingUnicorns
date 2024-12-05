use std::fs;

use uuid::Uuid;

use crate::config::home;

use crate::services::old_structs::OldBodyTypes;
use crate::services::structs::{BodyTypes, CollectionConfig, Options};

use super::old_structs::{BodyTypesConverter, OldCollectionConfig};
use super::structs::{Request, RequestOptions};

fn get_collections_path() -> String {
    let base_path = home::get();
    return format!("{base_path}/collections");
}

fn get_collection_path(collection_name: &str) -> String {
    let base_path = get_collections_path();
    return format!("{base_path}/{collection_name}");
}

pub fn get_collections() -> Vec<CollectionConfig> {
    let path = get_collections_path();
    let files = fs::read_dir(path).expect("Not a folder");
    let mut confs: Vec<CollectionConfig> = Vec::new();

    for file in files.map(|file| file.unwrap().file_name()) {
        let collection = read_collection(file.to_str().unwrap());
        confs.push(collection);
    }

    return confs;
}

pub fn write_collection(collection_name: &str, config: CollectionConfig) {
    let path = get_collection_path(format!("{collection_name}.json").as_str());
    let strigified_config = serde_json::to_string(&config).expect("Error while parsing json");
    let _ = fs::write(path, strigified_config);
}

pub fn delete_collection(collection_name: &str) {
    let path = get_collection_path(format!("{collection_name}.json").as_str());

    fs::remove_file(path).expect("Error while deleting file");
}

pub fn read_collection(collection_name: &str) -> CollectionConfig {
    let path = get_collection_path(collection_name);
    let collection_config_result = fs::read_to_string(path.clone());

    let collection_config = match collection_config_result {
        Ok(file) => file,
        Err(error) => panic!("Problem opening the file: {:?}", error),
    };

    let mut config: CollectionConfig =
        serde_json::from_str(&collection_config).expect("JSON was not well-formatted");

    for request in &mut config.requests {
        request.id = Some(Uuid::new_v4().to_string());

        if request.options.is_none() {
            println!("Add default options to request ");
            request.options = Some(RequestOptions {
                headers: Some(Vec::new()),
                body: BodyTypes {
                    form_data: Vec::<(String, Options)>::new(),
                    json: String::new(),
                },
                params: Some(Vec::new()),
            });
        }
    }

    match config.environments {
        Some(ref mut environments) => {
            for environment in environments {
                environment.id = Some(Uuid::new_v4().to_string());
            }
        }
        None => {}
    }

    return config;
}

pub fn migrate() {
    let path = get_collections_path();
    let files = fs::read_dir(path).expect("Not a folder");

    for file in files.map(|file| file.unwrap().file_name()) {
        let path = get_collection_path(file.to_str().unwrap());
        let collection_config_result = fs::read_to_string(path.clone());

        let collection_config = match collection_config_result {
            Ok(file) => file,
            Err(error) => panic!("Problem opening the file: {:?}", error),
        };
        let conf: OldCollectionConfig = match serde_json::from_str(&collection_config) {
            Ok(result) => result,
            Err(error) => {
                println!("Cannot migrate, {:?}", error);
                return;
            }
        };
        let mut new_conf = CollectionConfig {
            name: conf.name,
            environments: conf.environments,
            requests: Vec::<Request>::new(),
        };

        for request in conf.requests {
            match request.options {
                None => {}
                Some(options) => {
                    let mut new_request = Request {
                        name: request.name,
                        id: request.id,
                        method: request.method,
                        pre_request_script: request.pre_request_script,
                        test: request.test,
                        url: request.url,
                        options: Some(RequestOptions {
                            headers: options.headers.clone(),
                            params: options.params.clone(),
                            body: BodyTypes {
                                json: String::new(),
                                form_data: Vec::<(String, Options)>::new(),
                            },
                        }),
                    };

                    match options.body {
                        None => {}
                        Some(existing_body) => match existing_body {
                            BodyTypesConverter::V2OldBodyTypes(old_body) => match old_body {
                                OldBodyTypes::FormData(form_data) => {
                                    match &mut new_request.options {
                                        None => {}
                                        Some(new_options) => {
                                            for old_body_part in form_data {
                                                new_options.body.form_data.push((
                                                    old_body_part.0.clone(),
                                                    old_body_part.1.clone(),
                                                ));
                                            }
                                        }
                                    }
                                }
                                OldBodyTypes::Json(json_data) => match &mut new_request.options {
                                    None => {}
                                    Some(new_options) => new_options.body.json = json_data,
                                },
                                OldBodyTypes::Null => {}
                            },
                            BodyTypesConverter::FirstBodyType(old_body) => {
                                match &mut new_request.options {
                                    None => {}
                                    Some(new_options) => {
                                        for old_body_part in old_body {
                                            new_options.body.form_data.push((
                                                old_body_part.0.clone(),
                                                old_body_part.1.clone(),
                                            ));
                                        }
                                    }
                                }
                            }
                        },
                    }
                    new_conf.requests.push(new_request);
                }
            }
        }

        println!("We write this newConfig {:?}", new_conf);
        let collection_path = get_collection_path(file.to_str().unwrap());
        let strigified_config = serde_json::to_string(&new_conf).expect("Error while parsing json");
        let _ = fs::write(collection_path, strigified_config);
    }
}
