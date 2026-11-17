//! Update banner and the update commands, as in the suite's other apps.

use egui::{Spinner, TopBottomPanel};
use egui_phosphor::regular as icon;
use updater::{UpdateEvent, UpdateMemory, UpdateState, Updater};

use super::command::Command;

/// The updater plus what it remembers across runs.
pub struct Updates {
    pub updater: Updater,
    memory: UpdateMemory,
    /// The running check is the automatic one, to record once done.
    auto_check: bool,
}

/// A short message for the status line.
pub struct Notice {
    pub error: bool,
    pub text: String,
}

impl Updates {
    pub fn new(updater: Updater, memory: UpdateMemory) -> Self {
        Self {
            updater,
            memory,
            auto_check: false,
        }
    }

    /// The daily automatic check, unless one ran in the last 24 h.
    pub fn check_if_due(&mut self) {
        let memory = &self.memory;
        if updater::should_check(memory.last_check(), updater::unix_now()) {
            self.auto_check = true;
            let skipped = memory.skipped_version();
            self.updater.check(false, skipped.as_deref());
        }
    }

    pub fn check_now(&mut self) {
        self.updater.check(true, None);
    }

    pub fn skip(&mut self) {
        if let Some(version) = self.updater.skip() {
            self.memory.skip_version(&version);
        }
    }

    /// Results of the background work, as messages to show.
    pub fn poll(&mut self) -> Option<Notice> {
        let event = self.updater.poll();
        if self.auto_check && !matches!(self.updater.state, UpdateState::Checking) {
            self.auto_check = false;
            self.memory.record_check(updater::unix_now());
        }
        let (error, text) = match event? {
            UpdateEvent::UpToDate => (false, "sendingUnicorns is up to date.".to_owned()),
            UpdateEvent::Error(e) => (true, format!("Update: {e}")),
            // The banner shows these.
            UpdateEvent::Available(_) | UpdateEvent::Ready => return None,
        };
        Some(Notice { error, text })
    }

    pub fn banner(&self, ctx: &egui::Context, commands: &mut Vec<Command>) {
        let state = &self.updater.state;
        let shown = match state {
            UpdateState::Available(_) | UpdateState::Ready(_) => !self.updater.dismissed,
            UpdateState::Downloading(_) => true,
            _ => false,
        };
        if !shown {
            return;
        }
        TopBottomPanel::bottom("update").show(ctx, |ui| {
            ui.horizontal(|ui| match state {
                UpdateState::Available(info) => {
                    ui.label(format!(
                        "{} sendingUnicorns {} is available.",
                        icon::ARROW_CIRCLE_UP,
                        info.version
                    ));
                    if ui.button("Install").clicked() {
                        commands.push(Command::InstallUpdate);
                    }
                    if ui.button("Later").clicked() {
                        commands.push(Command::UpdateLater);
                    }
                    if ui.button("Skip this version").clicked() {
                        commands.push(Command::SkipUpdate);
                    }
                    ui.hyperlink_to("What's new", &info.page_url);
                }
                UpdateState::Downloading(info) => {
                    ui.add(Spinner::new());
                    ui.label(format!("Downloading sendingUnicorns {}…", info.version));
                }
                UpdateState::Ready(info) => {
                    ui.label(format!(
                        "{} sendingUnicorns {} is ready.",
                        icon::CHECK_CIRCLE,
                        info.version
                    ));
                    if ui.button("Restart now").clicked() {
                        commands.push(Command::RestartToUpdate);
                    }
                    if ui.button("Later").clicked() {
                        commands.push(Command::UpdateLater);
                    }
                }
                _ => {}
            });
        });
    }
}
