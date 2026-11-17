//! Sending Unicorns: a light and fast HTTP client.
//!
//! This crate is the presentation layer (egui) and the composition root that
//! wires the infrastructure adapters into the application use cases.

// No console window on Windows. Launched from a terminal (`cargo run`), the
// inherited handles still show the logs.
#![windows_subsystem = "windows"]

mod bootstrap;
mod ui;

use updater::AppInfo;

/// What the self-updater needs to know about this app.
pub static APP: AppInfo = AppInfo {
    name: "sendingUnicorns",
    version: env!("CARGO_PKG_VERSION"),
    manifest_url: "https://ajustor.github.io/sendingUnicorns/latest.json",
};

const HELP: &str = "\
sendingUnicorns — a light and fast HTTP client

USAGE:
    sendingUnicorns              Open the app
    sendingUnicorns --version    Print the version
    sendingUnicorns --help       Print this help

ENVIRONMENT:
    SU_DATA_DIR    Keep collections and settings in this folder
    RUST_LOG       Log level (error, warn, info, debug)";

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    // The only output of a CLI flag: printing is the point (R-VER-10).
    #[allow(clippy::print_stdout)]
    match std::env::args().nth(1).as_deref() {
        Some("--version" | "-V") => {
            println!("sendingUnicorns {}", APP.version);
            return Ok(());
        }
        Some("--help" | "-h") => {
            println!("{HELP}");
            return Ok(());
        }
        _ => {}
    }
    updater::cleanup_stale_staging(&APP);

    let viewport = egui::ViewportBuilder::default()
        .with_title(ui::APP_NAME)
        .with_app_id("sendingUnicorns")
        .with_inner_size([1360.0, 840.0])
        .with_min_inner_size([900.0, 520.0])
        .with_icon(window_icon());
    // The title bar is ours (gradient, tabs) and draws the window's buttons;
    // macOS keeps its own buttons, over it.
    let viewport = if cfg!(target_os = "macos") {
        viewport
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false)
    } else {
        viewport.with_decorations(false)
    };
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    let exit_slot = ui::ExitSlot::default();
    let slot = exit_slot.clone();
    eframe::run_native(
        ui::APP_NAME,
        options,
        Box::new(move |cc| Ok(Box::new(bootstrap::build(cc, slot)?))),
    )?;

    // The window is gone: apply a downloaded update if one was scheduled.
    let action = exit_slot
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take();
    if let Some(action) = action
        && let Err(e) = updater::run_exit_action(&action, &[])
    {
        log::error!("could not apply the update ({action:?}): {e}");
    }
    Ok(())
}

/// Title bar and taskbar icon (the .exe's own icon comes from build.rs).
fn window_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")).unwrap_or_default()
}
