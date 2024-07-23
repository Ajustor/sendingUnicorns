// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
mod config;
mod services;

use services::structs::RequestParams;

use crate::config::home;
use crate::services::{api_service, file_service, structs};

#[tauri::command]
#[specta::specta]
fn make_api_call(method: String, url: String, options: RequestParams) -> Result<String, String> {
    return tauri::async_runtime::block_on(async {
        let response = api_service::call(method, url, options).await;

        if response.is_err() {
            return Err(response.unwrap_err().to_string());
        }

        return Ok(response.unwrap());
    });
}

#[tauri::command]
#[specta::specta]
fn create_collection(collection_name: &str, config: structs::CollectionConfig) {
    file_service::write_collection(collection_name, config);
}

#[tauri::command]
#[specta::specta]
fn update_collection(collection_name: &str, config: structs::CollectionConfig) {
    file_service::delete_collection(collection_name);
    file_service::write_collection(collection_name, config);
}

#[tauri::command]
#[specta::specta]
fn get_collections() -> Vec<structs::CollectionConfig> {
    return file_service::get_collections();
}

// fn add_shortcut_plugin(builder: tauri::Builder<Wry>) -> tauri::Builder<Wry> {
//     return builder;
// }

fn init() -> std::io::Result<()> {
    let base_path = home::get();
    fs::create_dir_all(format!("{base_path}/collections"))?;
    Ok(())
}

fn main() {
    let config_creation = init();
    if config_creation.is_err() {
        return;
    }

    let (invoke_handler, register_events) = {
        // You can use `tauri_specta::js::builder` for exporting JS Doc instead of Typescript!`
        let builder = tauri_specta::ts::builder()
            .commands(tauri_specta::collect_commands![
                make_api_call,
                create_collection,
                get_collections,
                update_collection
            ])
            .events(tauri_specta::collect_events![]); // <- Each of your commands

        #[cfg(debug_assertions)] // <- Only export on non-release builds
        let builder = builder.path("../src/tauriApi.ts");

        builder.build().unwrap()
    };

    let mut ctx = tauri::generate_context!();

    tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_theme::init(ctx.config_mut()))
        .setup(|app| {
            #[cfg(desktop)]
            {
                println!("Start shortcuts register");

                use tauri::Manager;
                use tauri_plugin_global_shortcut::{Code, Modifiers, ShortcutState};
                println!("Start shortcuts register");

                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_shortcuts(["CommandOrControl+s"])?
                        .with_handler(|app, shortcut, event| {
                            println!("Shortcut is pressed {}", shortcut);
                            if event.state == ShortcutState::Pressed {
                                if shortcut.matches(Modifiers::CONTROL, Code::KeyS)
                                    || shortcut.matches(Modifiers::SUPER, Code::KeyS)
                                {
                                    let _ = app.emit("shortcut-event", "save");
                                }
                            }
                        })
                        .build(),
                )?;

                println!("Shortcuts registered");
            }
            register_events(app);
            Ok(())
        })
        .invoke_handler(invoke_handler)
        .run(ctx)
        .expect("error while running tauri application");
}
