//! The window: the title bar with the open requests, the address bar, the
//! collections on the left, the request's tabs in the middle, its response
//! on the right, a status bar below.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use application::collections::{
    CreateCollection, DeleteCollection, ExportCollection, ImportCollection, RenameCollection,
    SaveCollections,
};
use application::history::RecordHistory;
use application::runner::RunCollection;
use application::sending::SendRequest;
use application::settings::{ChangeSettings, ManageCookies};
use domain::{Auth, History, HistoryEntry, RequestId, Settings, Workspace};
use egui::{
    CentralPanel, Frame, Key, KeyboardShortcut, Margin, Modifiers, RichText, SidePanel,
    TextureHandle, TopBottomPanel, ViewportCommand,
};
use egui_phosphor::regular as icon;
use updater::ExitAction;

use super::command::Command;
use super::dialogs::{CodeWindow, Dialog, EnvironmentsWindow, ItemSettingsWindow, SettingsWindow};
use super::editor::{Editor, Inheritance};
use super::preview::Preview;
use super::response::{Exchange, ResponseView};
use super::runner::RunnerWindow;
use super::sender::Sender;
use super::sidebar::Sidebar;
use super::title_bar::TabInfo;
use super::toolbar::Location;
use super::updates::Updates;
use super::{auth, theme, title_bar, toolbar, window_frame};

/// Where the app leaves an update to apply once the window is gone.
pub type ExitSlot = Arc<Mutex<Option<ExitAction>>>;

/// Every use case the window runs, wired by the composition root.
#[derive(Debug)]
pub struct UseCases {
    pub create: CreateCollection,
    pub save: SaveCollections,
    pub rename: RenameCollection,
    pub delete: DeleteCollection,
    pub import: ImportCollection,
    pub export: ExportCollection,
    pub send: SendRequest,
    pub run: RunCollection,
    pub change_settings: ChangeSettings,
    pub cookies: ManageCookies,
    pub history: RecordHistory,
}

/// A line in the status bar.
#[derive(Debug)]
pub(super) struct Notice {
    pub error: bool,
    pub text: String,
}

pub struct SendingApp {
    pub(super) workspace: Workspace,
    pub(super) settings: Settings,
    pub(super) use_cases: UseCases,
    pub(super) history: History,
    /// The tabs, in order.
    pub(super) open: Vec<RequestId>,
    pub(super) active: Option<RequestId>,
    pub(super) exchanges: HashMap<RequestId, Exchange>,
    pub(super) sender: Sender,
    sidebar: Sidebar,
    editor: Editor,
    response: ResponseView,
    pub(super) dialog: Option<Dialog>,
    pub(super) environments: Option<EnvironmentsWindow>,
    pub(super) item_settings: Option<ItemSettingsWindow>,
    pub(super) settings_window: Option<SettingsWindow>,
    pub(super) code: Option<CodeWindow>,
    pub(super) runner: Option<RunnerWindow>,
    pub(super) notice: Option<Notice>,
    pub(super) updates: Updates,
    exit_slot: ExitSlot,
    preview: Preview,
    /// Unsaved changes were dealt with: the next close request closes.
    pub(super) may_close: bool,
    logo: Option<TextureHandle>,
}

impl std::fmt::Debug for SendingApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SendingApp")
            .field("open", &self.open)
            .field("active", &self.active)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for Updates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Updates").finish_non_exhaustive()
    }
}

impl SendingApp {
    #[allow(clippy::too_many_arguments)] // the composition root hands everything over once
    pub fn new(
        ctx: &egui::Context,
        workspace: Workspace,
        history: History,
        settings: Settings,
        use_cases: UseCases,
        mut updates: Updates,
        exit_slot: ExitSlot,
        preview: Preview,
        warnings: Vec<String>,
    ) -> Self {
        updates.check_if_due();
        for warning in &warnings {
            log::warn!("{warning}");
        }
        let first = workspace
            .collections()
            .iter()
            .find_map(|c| c.requests().first().map(|r| r.id));
        Self {
            workspace,
            settings,
            use_cases,
            history,
            open: first.into_iter().collect(),
            active: first,
            exchanges: HashMap::new(),
            sender: Sender::new(ctx.clone()),
            sidebar: Sidebar::default(),
            editor: Editor::default(),
            response: ResponseView::default(),
            dialog: None,
            environments: None,
            item_settings: None,
            settings_window: None,
            code: None,
            runner: None,
            notice: warnings
                .into_iter()
                .next()
                .map(|text| Notice { error: true, text }),
            updates,
            exit_slot,
            preview,
            may_close: false,
            logo: logo(ctx),
        }
    }

    pub(super) fn info(&mut self, text: impl Into<String>) {
        self.notice = Some(Notice {
            error: false,
            text: text.into(),
        });
    }

    pub(super) fn error(&mut self, text: impl Into<String>) {
        let text = text.into();
        log::warn!("{text}");
        self.notice = Some(Notice { error: true, text });
    }

    fn shortcuts(&self, ctx: &egui::Context, commands: &mut Vec<Command>) {
        let ctrl = |key| KeyboardShortcut::new(Modifiers::COMMAND, key);
        let ctrl_shift = |key| KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, key);
        let current = self.active.and_then(|id| {
            let c = self.workspace.collection_of(id)?;
            Some((c.name.clone(), c.ancestors(id).last().map(|f| f.id)))
        });
        ctx.input_mut(|i| {
            if i.consume_shortcut(&ctrl(Key::Enter)) {
                commands.push(Command::Send);
            }
            if i.consume_shortcut(&ctrl(Key::S)) {
                commands.push(Command::Save);
            }
            if i.consume_shortcut(&ctrl(Key::T)) {
                commands.push(Command::NewTab);
            }
            if i.consume_shortcut(&ctrl(Key::W)) {
                commands.push(Command::CloseActiveTab);
            }
            if i.consume_shortcut(&ctrl_shift(Key::Tab)) {
                commands.push(Command::CycleTabs(-1));
            }
            if i.consume_shortcut(&ctrl(Key::Tab)) {
                commands.push(Command::CycleTabs(1));
            }
            if let Some((collection, folder)) = current
                && i.consume_shortcut(&ctrl(Key::N))
            {
                commands.push(Command::NewRequest { collection, folder });
            }
        });
    }

    fn tabs(&self) -> Vec<TabInfo> {
        self.open
            .iter()
            .filter_map(|&id| {
                let found = self.workspace.find(id)?;
                Some(TabInfo {
                    id,
                    method: found.request.method,
                    name: found.request.name.clone(),
                    draft: found.collection.is_none(),
                    dirty: found
                        .collection
                        .is_some_and(|c| self.workspace.is_dirty(&c.name)),
                    sending: self.sender.sending(id).is_some(),
                })
            })
            .collect()
    }

    fn status_bar(&self, ctx: &egui::Context, commands: &mut Vec<Command>) {
        TopBottomPanel::bottom("status")
            .frame(Frame::side_top_panel(&ctx.style()).inner_margin(Margin::symmetric(10, 4)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let version = theme::hover_fill(ui, |ui| {
                        ui.add(
                            egui::Button::new(
                                RichText::new(format!("v{}", crate::APP.version)).small(),
                            )
                            .frame(false),
                        )
                    })
                    .on_hover_text("Check for updates");
                    if version.clicked() {
                        commands.push(Command::CheckForUpdates);
                    }
                    if theme::icon_button(ui, icon::GEAR_SIX, "Settings").clicked() {
                        commands.push(Command::OpenSettings);
                    }
                    if theme::icon_button(ui, icon::PLANET, "Global variables").clicked() {
                        commands.push(Command::ManageGlobals);
                    }
                    let runtime = self.workspace.runtime();
                    if !runtime.is_empty() {
                        let list: Vec<String> =
                            runtime.iter().map(|(k, v)| format!("{k} = {v}")).collect();
                        ui.label(
                            RichText::new(format!("{} {} runtime", icon::LIGHTNING, runtime.len()))
                                .small()
                                .color(theme::weak(ui)),
                        )
                        .on_hover_text(list.join("\n"));
                    }
                    if self.workspace.has_unsaved_changes() {
                        ui.label(
                            RichText::new("● unsaved (Ctrl+S)")
                                .small()
                                .color(theme::PINK),
                        );
                    }
                    if let Some(notice) = &self.notice {
                        let color = if notice.error {
                            theme::error(ui)
                        } else {
                            theme::weak(ui)
                        };
                        ui.label(RichText::new(&notice.text).small().color(color));
                    }
                });
            });
    }

    /// The address bar, the request's tabs and its response.
    fn request_view(&mut self, ctx: &egui::Context, commands: &mut Vec<Command>) {
        let Some(found) = self.active.and_then(|id| self.workspace.find(id)) else {
            self.welcome(ctx, commands);
            return;
        };
        let id = found.request.id;
        let mut request = found.request.clone();
        let collection = found.collection;
        let location = Location {
            collection,
            folders: collection.map_or_else(Vec::new, |c| {
                c.ancestors(id).iter().map(|f| f.name.clone()).collect()
            }),
            environment: collection
                .and_then(|c| self.workspace.active_environment(&c.name))
                .map(|e| e.name.as_str()),
        };
        let inheritance = collection.map_or_else(Inheritance::default, |c| Inheritance {
            auth: Some(inherited_auth(c, id)),
            headers: c.inherited_headers(id),
        });
        let variables = self.workspace.variables(id);
        let sending = self.sender.sending(id);

        let mut changed = toolbar::show(
            ctx,
            &mut request,
            &location,
            &variables,
            sending.is_some(),
            commands,
        );
        CentralPanel::default()
            .frame(Frame::central_panel(&ctx.style()).inner_margin(Margin::same(12)))
            .show(ctx, |ui| {
                let width = ui.available_width();
                SidePanel::right("response")
                    .resizable(true)
                    .default_width(width / 2.0)
                    .width_range(280.0..=(width - 300.0).max(300.0))
                    .frame(Frame::NONE.inner_margin(Margin {
                        left: 14,
                        ..Margin::ZERO
                    }))
                    .show_inside(ui, |ui| {
                        self.response.show(
                            ui,
                            self.exchanges.get(&id),
                            sending,
                            &mut self.preview,
                            commands,
                        );
                    });
                CentralPanel::default()
                    .frame(Frame::NONE)
                    .show_inside(ui, |ui| {
                        changed |= self.editor.show(ui, &mut request, &variables, &inheritance);
                    });
            });
        if changed && let Some(stored) = self.workspace.edit_request(id) {
            *stored = request;
        }
    }

    fn welcome(&self, ctx: &egui::Context, commands: &mut Vec<Command>) {
        CentralPanel::default().show(ctx, |ui| {
            ui.add_space(ui.available_height() / 6.0);
            ui.vertical_centered(|ui| {
                if let Some(logo) = &self.logo {
                    ui.add(
                        egui::Image::new(logo)
                            .fit_to_exact_size(egui::vec2(112.0, 112.0))
                            .corner_radius(24.0),
                    );
                }
                ui.add_space(8.0);
                let mut job = egui::text::LayoutJob::default();
                let strong = ui.visuals().strong_text_color();
                job.append(
                    "sending",
                    0.0,
                    egui::TextFormat::simple(egui::FontId::proportional(34.0), strong),
                );
                job.append(
                    "Unicorns",
                    0.0,
                    egui::TextFormat::simple(egui::FontId::proportional(34.0), theme::accent(ui)),
                );
                ui.label(job);
                ui.label(
                    RichText::new("A light and fast HTTP client")
                        .size(16.0)
                        .color(theme::weak(ui)),
                );
                ui.add_space(18.0);
                let big = |text: String| egui::Button::new(text).min_size(egui::vec2(260.0, 34.0));
                let actions = [
                    (
                        format!("{}  New request   Ctrl+T", icon::PLUS),
                        Command::NewTab,
                    ),
                    (
                        format!("{}  New collection", icon::FOLDER_PLUS),
                        Command::NewCollection,
                    ),
                    (
                        format!("{}  Import (ours, Postman)…", icon::DOWNLOAD_SIMPLE),
                        Command::Import,
                    ),
                    (
                        format!("{}  Paste a cURL command…", icon::TERMINAL_WINDOW),
                        Command::ImportCurl,
                    ),
                ];
                for (label, command) in actions {
                    if ui.add(big(label)).clicked() {
                        commands.push(command);
                    }
                }
            });
        });
    }

    /// Closing with unsaved changes asks first.
    fn guard_close(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested())
            && !self.may_close
            && self.workspace.has_unsaved_changes()
        {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            self.dialog = Some(Dialog::Unsaved);
        }
    }

    /// A response came: its checks run, it goes in the history.
    fn receive(&mut self) {
        let Some((id, prepared, result)) = self.sender.poll() else {
            return;
        };
        let (after, status, elapsed_ms) = match &result {
            Ok(response) => (
                SendRequest::after(&mut self.workspace, id, response),
                Some(response.status),
                u64::try_from(response.elapsed.as_millis()).unwrap_or(u64::MAX),
            ),
            Err(_) => (domain::AfterResponse::default(), None, 0),
        };
        if let Some(found) = self.workspace.find(id) {
            let entry = HistoryEntry {
                at: updater::unix_now(),
                request: found.request.clone(),
                status,
                elapsed_ms,
            };
            if let Err(e) = self.use_cases.history.execute(&mut self.history, entry) {
                log::warn!("history: {e}");
            }
        }
        let exchange = Exchange::new(
            prepared.url,
            result.map_err(|e| e.to_string()),
            prepared.missing_variables,
            after,
        );
        self.exchanges.insert(id, exchange);
    }
}

/// "Inherits Bearer token from folder users".
fn inherited_auth(c: &domain::Collection, id: RequestId) -> String {
    c.ancestors(id)
        .iter()
        .rev()
        .find(|f| f.auth != Auth::Inherit)
        .map_or_else(
            || auth::describe(&c.auth, &format!("collection {}", c.name)),
            |f| auth::describe(&f.auth, &format!("folder {}", f.name)),
        )
}

/// The app's icon, for the title bar and the welcome screen.
fn logo(ctx: &egui::Context) -> Option<TextureHandle> {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../../assets/icon.png")).ok()?;
    let size = [
        usize::try_from(icon.width).ok()?,
        usize::try_from(icon.height).ok()?,
    ];
    let image = egui::ColorImage::from_rgba_unmultiplied(size, &icon.rgba);
    Some(ctx.load_texture("logo", image, egui::TextureOptions::LINEAR))
}

impl eframe::App for SendingApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.preview.begin_frame(ctx);
        window_frame::resize_edges(ctx);
        self.guard_close(ctx);
        self.receive();
        if let Some(notice) = self.updates.poll() {
            self.notice = Some(Notice {
                error: notice.error,
                text: notice.text,
            });
        }
        if let Some(vars) = self.runner.as_mut().and_then(RunnerWindow::take_variables) {
            for (name, value) in vars {
                self.workspace.set_runtime(&name, value);
            }
        }
        // Tabs of requests that are gone.
        self.open.retain(|&id| self.workspace.find(id).is_some());
        if self.active.is_some_and(|id| !self.open.contains(&id)) {
            self.active = self.open.last().copied();
        }

        let mut commands = Vec::new();
        if self.dialog.is_none() {
            self.shortcuts(ctx, &mut commands);
        }
        title_bar::show(
            ctx,
            self.logo.as_ref(),
            &self.tabs(),
            self.active,
            &mut commands,
        );
        self.updates.banner(ctx, &mut commands);
        self.status_bar(ctx, &mut commands);
        self.sidebar.show(
            ctx,
            &self.workspace,
            &self.history,
            self.active,
            &mut commands,
        );
        self.request_view(ctx, &mut commands);
        self.windows(ctx, &mut commands);
        self.dialogs(ctx);

        for command in commands {
            self.run(ctx, command);
        }
        // A native view covers egui: hidden under windows, dialogs, menus.
        let covered = self.dialog.is_some()
            || self.environments.is_some()
            || self.item_settings.is_some()
            || self.settings_window.is_some()
            || self.code.is_some()
            || self.runner.is_some()
            || ctx.memory(egui::Memory::any_popup_open);
        self.preview.end_frame(covered);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // A downloaded update still applies on a plain quit (MSI).
        self.updates.updater.schedule_on_quit();
        self.exit_slot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone_from(&self.updates.updater.exit_action);
    }
}
