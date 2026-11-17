//! The collection runner's window: sends a collection's (or folder's)
//! requests in order on a background thread, and lists how each went.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::thread;

use application::runner::{RunCollection, RunEvent, RunStep};
use domain::{CollectionName, RequestId, Settings, Workspace};
use egui::{RichText, Ui};
use egui_phosphor::regular as icon;

use super::command::Command;
use super::theme;

pub struct RunnerWindow {
    title: String,
    events: Receiver<RunEvent>,
    stop: Arc<AtomicBool>,
    total: usize,
    sending: Option<(usize, String)>,
    steps: Vec<RunStep>,
    /// The runtime variables the run set, once it is over.
    pub finished: Option<Vec<(String, String)>>,
    merged: bool,
}

impl std::fmt::Debug for RunnerWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunnerWindow")
            .field("title", &self.title)
            .field("steps", &self.steps.len())
            .finish_non_exhaustive()
    }
}

impl RunnerWindow {
    /// Starts the run on its own copy of the workspace.
    pub fn start(
        run: RunCollection,
        workspace: Workspace,
        collection: CollectionName,
        folder: Option<RequestId>,
        title: String,
        settings: Settings,
        repaint: egui::Context,
    ) -> Self {
        let (tx, events) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        thread::spawn(move || {
            run.execute(
                workspace,
                &collection,
                folder,
                &settings,
                &|| stopped.load(Ordering::Relaxed),
                &mut |event| {
                    // The window may be closed: nobody is left to tell.
                    let _ = tx.send(event);
                    repaint.request_repaint();
                },
            );
        });
        Self {
            title,
            events,
            stop,
            total: 0,
            sending: None,
            steps: Vec::new(),
            finished: None,
            merged: false,
        }
    }

    /// The runtime variables to keep, once (when the run is over).
    pub fn take_variables(&mut self) -> Option<Vec<(String, String)>> {
        if self.merged {
            return None;
        }
        let vars = self.finished.clone()?;
        self.merged = true;
        Some(vars)
    }

    fn poll(&mut self) {
        while let Ok(event) = self.events.try_recv() {
            match event {
                RunEvent::Started(total) => self.total = total,
                RunEvent::Sending(index, name) => self.sending = Some((index, name)),
                RunEvent::Step(step) => self.steps.push(step),
                RunEvent::Finished(vars) => {
                    self.sending = None;
                    self.finished = Some(vars);
                }
            }
        }
    }

    /// `false` once closed (the run is stopped).
    pub fn show(&mut self, ctx: &egui::Context, commands: &mut Vec<Command>) -> bool {
        self.poll();
        let mut open = true;
        egui::Window::new(format!("{} Run — {}", icon::PLAY, self.title))
            .open(&mut open)
            .default_size([640.0, 440.0])
            .collapsible(false)
            .show(ctx, |ui| self.contents(ui, commands));
        if !open {
            self.stop.store(true, Ordering::Relaxed);
        }
        open
    }

    fn contents(&mut self, ui: &mut Ui, commands: &mut Vec<Command>) {
        let passed = self.steps.iter().filter(|s| s.passed()).count();
        let failed = self.steps.len() - passed;
        ui.horizontal(|ui| {
            #[allow(clippy::cast_precision_loss)] // a progress bar
            let progress = if self.total == 0 {
                1.0
            } else {
                self.steps.len() as f32 / self.total as f32
            };
            ui.add(
                egui::ProgressBar::new(progress)
                    .desired_width(200.0)
                    .fill(theme::accent(ui).gamma_multiply(0.7)),
            );
            theme::badge(ui, &format!("{passed} passed"), theme::success(ui), 0.0);
            if failed > 0 {
                theme::badge(ui, &format!("{failed} failed"), theme::error(ui), 0.0);
            }
            if self.finished.is_none() {
                if ui.button(format!("{} Stop", icon::STOP)).clicked() {
                    self.stop.store(true, Ordering::Relaxed);
                }
            } else {
                ui.label(RichText::new("Done").color(theme::weak(ui)));
            }
        });
        if let Some((index, name)) = &self.sending {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(format!("{} / {} — {name}", index + 1, self.total));
            });
        }
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                for step in &self.steps {
                    step_row(ui, step, commands);
                }
            });
    }
}

fn step_row(ui: &mut Ui, step: &RunStep, commands: &mut Vec<Command>) {
    egui::CollapsingHeader::new(
        RichText::new(format!(
            "{}  {}  {}",
            if step.passed() {
                icon::CHECK_CIRCLE
            } else {
                icon::X_CIRCLE
            },
            step.method,
            step.name
        ))
        .color(if step.passed() {
            theme::success(ui)
        } else {
            theme::error(ui)
        }),
    )
    .id_salt(("step", step.id))
    .show(ui, |ui| {
        ui.horizontal(|ui| {
            match &step.outcome {
                Ok(status) => {
                    ui.label(format!("{status} · {} ms", step.elapsed_ms));
                }
                Err(e) => {
                    ui.label(RichText::new(e).color(theme::error(ui)));
                }
            }
            if ui
                .small_button(format!("{} Open", icon::ARROW_SQUARE_OUT))
                .clicked()
            {
                commands.push(Command::Open(step.id));
            }
        });
        ui.label(
            RichText::new(&step.url)
                .small()
                .monospace()
                .color(theme::weak(ui)),
        );
        for result in &step.after.results {
            let (glyph, color) = if result.passed {
                (icon::CHECK, theme::success(ui))
            } else {
                (icon::X, theme::error(ui))
            };
            ui.horizontal(|ui| {
                ui.label(RichText::new(glyph).color(color));
                ui.label(RichText::new(&result.label).monospace());
                if !result.passed {
                    ui.label(
                        RichText::new(format!("got {}", result.actual))
                            .small()
                            .color(theme::weak(ui)),
                    );
                }
            });
        }
        for (name, value) in &step.after.variables {
            ui.label(
                RichText::new(format!("{name} = {value}"))
                    .small()
                    .monospace(),
            );
        }
    });
}
