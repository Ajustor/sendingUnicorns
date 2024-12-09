// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
mod config;
mod services;

use services::structs::{BodyTypesEnum, RequestParams};
use specta_typescript::Typescript;
use tauri::menu::{CheckMenuItem, IconMenuItem, MenuBuilder, MenuItem, SubmenuBuilder};
use tauri::{Emitter, EventTarget};
use tauri_specta::{collect_commands, Builder};

use crate::config::home;
use crate::services::{api_service, file_service, structs};

#[tauri::command]
#[specta::specta]
fn make_api_call(
    method: String,
    url: String,
    options: RequestParams,
    body_type: BodyTypesEnum,
) -> Result<String, String> {
    return tauri::async_runtime::block_on(async {
        let response = api_service::call(method, url, options, body_type).await;

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
    migrate()?;
    Ok(())
}

fn migrate() -> std::io::Result<()> {
    file_service::migrate();
    Ok(())
}

fn main() {
    let config_creation = init();
    if config_creation.is_err() {
        return;
    }

    let builder = Builder::<tauri::Wry>::new()
        // Then register them (separated by a comma)
        .commands(collect_commands![
            make_api_call,
            create_collection,
            get_collections,
            update_collection
        ]);

    builder
        .export(Typescript::default(), "../src/tauriApi.ts")
        .expect("Failed to export typescript bindings");

    let mut ctx = tauri::generate_context!("./tauri.conf.json");

    tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_theme::init(ctx.config_mut()))
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            let handle = app.handle();

            builder.mount_events(app);

            let theme_change = IconMenuItem::with_id(
                app,
                "toggle-theme",
                "Switch theme",
                true,
                Some(app.default_window_icon().cloned().unwrap()),
                Some("cmdOrControl+T"),
            )?;
            let view_submenu = SubmenuBuilder::new(handle, "View")
                .items(&[&theme_change])
                .build()?;

            let save_button =
                MenuItem::with_id(handle, "save", "Save", true, Some("cmdOrControl+S"))?;
            let file_submenu = SubmenuBuilder::new(handle, "File")
                .item(&save_button)
                .items(&[&CheckMenuItem::new(
                    handle,
                    "CheckMenuItem 1",
                    true,
                    true,
                    None::<&str>,
                )?])
                .separator()
                .cut()
                .copy()
                .paste()
                .separator()
                // .text("item2", "MenuItem 2")
                // .check("checkitem2", "CheckMenuItem 2")
                // .icon(
                //     "iconitem2",
                //     "IconMenuItem 2",
                //     app.default_window_icon().cloned().unwrap(),
                // )
                .build()?;

            let menu = MenuBuilder::new(app)
                .items(&[&file_submenu, &view_submenu])
                .build()?;

            app.set_menu(menu)?;

            app.on_menu_event(move |handler, event| {
                if event.id() == theme_change.id() {
                    let _ = handler.emit_to(EventTarget::app(), "toggle-theme", {});
                }

                if event.id() == save_button.id() {
                    let _ = handler.emit_to(EventTarget::app(), "save", {});
                }
            });

            Ok(())
        })
        .run(ctx)
        .expect("error while running tauri application");
}
