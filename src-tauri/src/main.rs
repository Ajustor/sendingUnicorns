// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
mod config;
mod services;

use crate::config::home;
use crate::services::api_service;

fn init() -> std::io::Result<()> {
    let base_path = home::get();
    fs::create_dir_all(base_path.clone())?;
    Ok(())
}

#[tauri::command]
async fn make_api_call(method: String, url: String) -> String {
    return tokio::spawn(async move {
        let raw_rdo_schema = api_service::call(method, url).await;
        return raw_rdo_schema;
    })
    .await
    .unwrap();
}

#[tokio::main]

async fn main() {
    let config_creation = init();
    if config_creation.is_err() {
        return;
    }

    tauri::async_runtime::set(tokio::runtime::Handle::current());

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![make_api_call])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
