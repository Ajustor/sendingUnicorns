// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
mod config;
mod services;

use crate::config::home;
use crate::services::{api_service, file_service, structs};

fn init() -> std::io::Result<()> {
    let base_path = home::get();
    fs::create_dir_all(format!("{base_path}/collections"))?;
    Ok(())
}

#[tauri::command]
fn make_api_call(method: String, url: String) -> String {
    return tauri::async_runtime::block_on(async {
        return api_service::call(method, url).await;
    });
}

#[tauri::command]
fn create_collection(collection_name: &str, config: structs::CollectionConfig) {
    file_service::write_collection(collection_name, config);
}

#[tauri::command]
fn update_collection(collection_name: &str, config: structs::CollectionConfig) {
    file_service::delete_collection(collection_name);
    file_service::write_collection(collection_name, config);
}

#[tauri::command]
fn get_collections() -> Vec<structs::CollectionConfig> {
    return file_service::get_collections();
}

fn main() {
    let config_creation = init();
    if config_creation.is_err() {
        return;
    }

    let mut ctx = tauri::generate_context!();

    tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_theme::init(ctx.config_mut()))
        .invoke_handler(tauri::generate_handler![make_api_call, create_collection, get_collections, update_collection])
        .run(ctx)
        .expect("error while running tauri application");
}
