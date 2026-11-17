//! Presentation layer: the HTTP client drawn with egui. Widgets turn clicks
//! into [`command::Command`]s, run once per frame through the use cases.

mod actions;
mod app;
mod auth;
mod command;
mod dialogs;
mod editor;
mod highlight;
mod response;
mod runner;
mod sender;
mod sidebar;
mod system_font;
mod table;
pub mod theme;
mod title_bar;
mod toolbar;
mod updates;
mod window_frame;

pub use app::{ExitSlot, SendingApp, UseCases};
pub use updates::Updates;

pub const APP_NAME: &str = "Sending Unicorns";
