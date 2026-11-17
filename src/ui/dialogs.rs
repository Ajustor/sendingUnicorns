//! Modal dialogs (names, confirmations, unsaved changes, cURL, saving a
//! draft) and the windows: environments, globals, settings, collection and
//! folder settings, cURL export.

use domain::{Auth, CollectionName, Cookie, KeyValues, RequestId, Settings, Variables, Workspace};
use egui::{Key, Modal, RichText, TextEdit, Ui};
use egui_phosphor::regular as icon;

use super::auth;
use super::editor::{heading, hint};
use super::table::{Columns, key_values};
use super::theme;

/// What a name is asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamePurpose {
    NewCollection,
    RenameCollection(CollectionName),
    NewRequest {
        collection: CollectionName,
        folder: Option<RequestId>,
    },
    NewFolder {
        collection: CollectionName,
        parent: Option<RequestId>,
    },
    /// A request or a folder.
    Rename(RequestId),
    NewEnvironment(CollectionName),
    RenameEnvironment(CollectionName, String),
}

impl NamePurpose {
    fn title(&self) -> &'static str {
        match self {
            Self::NewCollection => "New collection",
            Self::RenameCollection(_) | Self::Rename(_) | Self::RenameEnvironment(..) => "Rename",
            Self::NewRequest { .. } => "New request",
            Self::NewFolder { .. } => "New folder",
            Self::NewEnvironment(_) => "New environment",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteTarget {
    Collection(CollectionName),
    /// A request or a folder.
    Item(RequestId),
    Environment(CollectionName, String),
}

#[derive(Debug)]
pub enum Dialog {
    Name {
        purpose: NamePurpose,
        text: String,
        error: Option<String>,
    },
    Delete {
        target: DeleteTarget,
        what: String,
    },
    /// Quitting with unsaved collections.
    Unsaved,
    /// Closing a draft's tab drops it.
    CloseDraft(RequestId),
    /// A curl command to paste.
    Curl {
        text: String,
        error: Option<String>,
    },
    /// Which collection a draft goes to, under which name.
    SaveDraft {
        id: RequestId,
        name: String,
        collection: Option<CollectionName>,
        error: Option<String>,
    },
}

#[derive(Debug)]
pub enum Outcome {
    Open,
    Cancelled,
    Name(NamePurpose, String),
    Delete(DeleteTarget),
    SaveAndQuit,
    Quit,
    DropDraft(RequestId),
    Curl(String),
    SaveDraft {
        id: RequestId,
        name: String,
        collection: CollectionName,
    },
}

impl Dialog {
    pub fn name(purpose: NamePurpose, text: impl Into<String>) -> Self {
        Self::Name {
            purpose,
            text: text.into(),
            error: None,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, workspace: &Workspace) -> Outcome {
        let mut outcome = Outcome::Open;
        let modal = Modal::new(egui::Id::new("dialog")).show(ctx, |ui| {
            ui.set_width(420.0);
            match self {
                Self::Name {
                    purpose,
                    text,
                    error,
                } => {
                    ui.heading(purpose.title());
                    ui.add_space(8.0);
                    let field = ui.add(TextEdit::singleline(text).desired_width(f32::INFINITY));
                    field.request_focus();
                    error_line(ui, error.as_deref());
                    ui.add_space(8.0);
                    let enter = field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                    if buttons(ui, "OK", &mut outcome) || enter {
                        outcome = Outcome::Name(purpose.clone(), text.clone());
                    }
                }
                Self::Delete { target, what } => {
                    ui.heading(format!("Delete {what}?"));
                    ui.add_space(8.0);
                    ui.label("This cannot be undone once saved.");
                    ui.add_space(8.0);
                    if buttons(ui, "Delete", &mut outcome) {
                        outcome = Outcome::Delete(target.clone());
                    }
                }
                Self::Unsaved => {
                    ui.heading("Unsaved changes");
                    ui.add_space(8.0);
                    ui.label(
                        "Some collections or global variables have changes that are not saved.",
                    );
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Save and quit").clicked() {
                            outcome = Outcome::SaveAndQuit;
                        }
                        if ui.button("Quit without saving").clicked() {
                            outcome = Outcome::Quit;
                        }
                        if ui.button("Cancel").clicked() {
                            outcome = Outcome::Cancelled;
                        }
                    });
                }
                Self::CloseDraft(id) => {
                    ui.heading("Close the draft?");
                    ui.add_space(8.0);
                    ui.label(
                        "It is in no collection: closing it drops it. Ctrl+S saves it in one.",
                    );
                    ui.add_space(8.0);
                    if buttons(ui, "Drop it", &mut outcome) {
                        outcome = Outcome::DropDraft(*id);
                    }
                }
                Self::Curl { text, error } => outcome = curl(ui, text, error.as_deref()),
                Self::SaveDraft {
                    id,
                    name,
                    collection,
                    error,
                } => outcome = save_draft(ui, workspace, *id, name, collection, error),
            }
        });
        if modal.should_close() && matches!(outcome, Outcome::Open) {
            outcome = Outcome::Cancelled;
        }
        outcome
    }

    /// Shows why it was refused; the dialog stays open.
    pub fn refuse(&mut self, why: String) {
        match self {
            Self::Name { error, .. } | Self::Curl { error, .. } | Self::SaveDraft { error, .. } => {
                *error = Some(why);
            }
            _ => {}
        }
    }
}

fn curl(ui: &mut Ui, text: &mut String, error: Option<&str>) -> Outcome {
    let mut outcome = Outcome::Open;
    ui.heading(format!("{} Import a cURL command", icon::TERMINAL_WINDOW));
    hint(
        ui,
        "Paste a command (a browser's \"Copy as cURL\" works): it opens as a new request.",
    );
    let field = ui.add(
        TextEdit::multiline(text)
            .code_editor()
            .desired_rows(8)
            .desired_width(f32::INFINITY)
            .hint_text("curl 'https://api.example.com/users' -H 'accept: application/json'"),
    );
    field.request_focus();
    error_line(ui, error);
    ui.add_space(8.0);
    if buttons(ui, "Import", &mut outcome) {
        outcome = Outcome::Curl(text.clone());
    }
    outcome
}

fn save_draft(
    ui: &mut Ui,
    workspace: &Workspace,
    id: RequestId,
    name: &mut String,
    collection: &mut Option<CollectionName>,
    error: &mut Option<String>,
) -> Outcome {
    let mut outcome = Outcome::Open;
    ui.heading(format!("{} Save the request", icon::FLOPPY_DISK));
    ui.add_space(8.0);
    egui::Grid::new("save draft")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label("Name");
            ui.add(TextEdit::singleline(name).desired_width(280.0));
            ui.end_row();
            ui.label("Collection");
            egui::ComboBox::from_id_salt("draft collection")
                .width(280.0)
                .selected_text(
                    collection
                        .as_ref()
                        .map_or("Choose…", CollectionName::as_str),
                )
                .show_ui(ui, |ui| {
                    for c in workspace.collections() {
                        ui.selectable_value(collection, Some(c.name.clone()), c.name.as_str());
                    }
                });
            ui.end_row();
        });
    if workspace.collections().is_empty() {
        hint(ui, "Create a collection first (the + of the side panel).");
    }
    error_line(ui, error.as_deref());
    ui.add_space(8.0);
    if buttons(ui, "Save", &mut outcome) {
        if let Some(c) = collection {
            outcome = Outcome::SaveDraft {
                id,
                name: name.clone(),
                collection: c.clone(),
            };
        } else {
            *error = Some("Choose a collection.".into());
        }
    }
    outcome
}

fn error_line(ui: &mut Ui, error: Option<&str>) {
    if let Some(error) = error {
        ui.label(RichText::new(error).color(theme::error(ui)));
    }
}

/// OK and Cancel; whether OK was clicked.
fn buttons(ui: &mut Ui, ok: &str, outcome: &mut Outcome) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        clicked = ui.button(ok).clicked();
        if ui.button("Cancel").clicked() {
            *outcome = Outcome::Cancelled;
        }
    });
    clicked
}

/// What the environments window asks of the app.
#[derive(Debug)]
pub enum EnvironmentAction {
    Add(CollectionName),
    Rename(CollectionName, String),
    Delete(CollectionName, String),
}

/// A collection's environments and their variables, or the globals.
#[derive(Debug)]
pub struct EnvironmentsWindow {
    /// `None`: the global variables.
    pub collection: Option<CollectionName>,
    selected: Option<String>,
}

impl EnvironmentsWindow {
    pub fn new(collection: CollectionName, selected: Option<String>) -> Self {
        Self {
            collection: Some(collection),
            selected,
        }
    }

    pub fn globals() -> Self {
        Self {
            collection: None,
            selected: None,
        }
    }

    /// `false` once closed.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        workspace: &mut Workspace,
        actions: &mut Vec<EnvironmentAction>,
    ) -> bool {
        let mut open = true;
        let Some(name) = self.collection.clone() else {
            let mut globals = workspace.globals().clone();
            egui::Window::new(format!("{} Global variables", icon::PLANET))
                .open(&mut open)
                .default_size([620.0, 360.0])
                .collapsible(false)
                .show(ctx, |ui| {
                    hint(
                        ui,
                        "Seen by every collection; a collection's or environment's variable wins.",
                    );
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        variables_table(ui, "globals", &mut globals);
                    });
                });
            if &globals != workspace.globals() {
                *workspace.edit_globals() = globals;
            }
            return open;
        };
        let Some(collection) = workspace.collection(&name) else {
            return false;
        };
        let names: Vec<String> = collection
            .environments
            .iter()
            .map(|e| e.name.clone())
            .collect();
        if self.selected.as_ref().is_none_or(|s| !names.contains(s)) {
            self.selected = names.first().cloned();
        }
        let mut variables = self
            .selected
            .as_ref()
            .and_then(|s| collection.environment(s))
            .map(|e| e.variables.clone());
        let before = variables.clone();

        egui::Window::new(format!("{} Environments — {name}", icon::GLOBE))
            .open(&mut open)
            .default_size([680.0, 400.0])
            .collapsible(false)
            .show(ctx, |ui| {
                hint(
                    ui,
                    "Use a variable as {{name}} in addresses, headers, bodies and auth.",
                );
                egui::SidePanel::left("environment list")
                    .resizable(false)
                    .exact_width(170.0)
                    .show_inside(ui, |ui| {
                        for env in &names {
                            if theme::pill(ui, self.selected.as_ref() == Some(env), env).clicked() {
                                self.selected = Some(env.clone());
                            }
                        }
                        ui.add_space(6.0);
                        if ui.button(format!("{} Add", icon::PLUS)).clicked() {
                            actions.push(EnvironmentAction::Add(name.clone()));
                        }
                        if let Some(env) = &self.selected {
                            ui.horizontal(|ui| {
                                if theme::icon_button(ui, icon::PENCIL_SIMPLE, "Rename").clicked() {
                                    actions
                                        .push(EnvironmentAction::Rename(name.clone(), env.clone()));
                                }
                                if theme::icon_button(ui, icon::TRASH, "Delete").clicked() {
                                    actions
                                        .push(EnvironmentAction::Delete(name.clone(), env.clone()));
                                }
                            });
                        }
                    });
                egui::CentralPanel::default().show_inside(ui, |ui| match variables.as_mut() {
                    Some(vars) => {
                        egui::ScrollArea::vertical()
                            .show(ui, |ui| variables_table(ui, "variables", vars));
                    }
                    None => {
                        ui.label(
                            RichText::new("No environment yet: add one.").color(theme::weak(ui)),
                        );
                    }
                });
            });

        if variables != before
            && let (Some(vars), Some(env)) = (variables, &self.selected)
            && let Some(environment) = workspace.edit(&name).and_then(|c| c.environment_mut(env))
        {
            environment.variables = vars;
        }
        open
    }
}

fn variables_table(ui: &mut Ui, id: &str, rows: &mut KeyValues) -> bool {
    let columns = Columns {
        secrets: true,
        ..Columns::new("Variable", "Value")
    };
    key_values(ui, id, rows, &Variables::default(), columns)
}

/// A collection's or folder's settings window.
#[derive(Debug, Clone)]
pub enum ItemSettings {
    Collection(CollectionName),
    Folder(RequestId),
}

#[derive(Debug)]
pub struct ItemSettingsWindow {
    pub target: ItemSettings,
    tab: usize,
}

impl ItemSettingsWindow {
    pub fn new(target: ItemSettings) -> Self {
        Self { target, tab: 0 }
    }

    /// `false` once closed.
    pub fn show(&mut self, ctx: &egui::Context, workspace: &mut Workspace) -> bool {
        match self.target.clone() {
            ItemSettings::Collection(name) => self.collection(ctx, workspace, &name),
            ItemSettings::Folder(id) => self.folder(ctx, workspace, id),
        }
    }

    fn collection(
        &mut self,
        ctx: &egui::Context,
        workspace: &mut Workspace,
        name: &CollectionName,
    ) -> bool {
        let mut open = true;
        let Some(c) = workspace.collection(name) else {
            return false;
        };
        let (mut auth_, mut headers, mut variables, mut docs) = (
            c.auth.clone(),
            c.headers.clone(),
            c.variables.clone(),
            c.docs.clone(),
        );
        let vars = workspace.variables_of_collection(name);
        egui::Window::new(format!("{} {name} — settings", icon::FOLDERS))
            .open(&mut open)
            .default_size([640.0, 420.0])
            .collapsible(false)
            .show(ctx, |ui| {
                self.tabs(ui, &["Auth", "Headers", "Variables", "Docs"]);
                egui::ScrollArea::vertical().show(ui, |ui| match self.tab {
                    0 => {
                        hint(ui, "Used by every request set to inherit.");
                        auth::editor(ui, &mut auth_, None, &vars);
                    }
                    1 => {
                        hint(ui, "Sent with every request, before its own headers.");
                        key_values(
                            ui,
                            "c headers",
                            &mut headers,
                            &vars,
                            Columns::new("Header", "Value"),
                        );
                    }
                    2 => {
                        hint(
                            ui,
                            "Shared by every environment; an environment's variable wins.",
                        );
                        variables_table(ui, "c variables", &mut variables);
                    }
                    _ => {
                        ui.add(
                            TextEdit::multiline(&mut docs)
                                .hint_text("What this API is, how to get a token…")
                                .desired_rows(14)
                                .desired_width(f32::INFINITY),
                        );
                    }
                });
            });
        let changed =
            auth_ != c.auth || headers != c.headers || variables != c.variables || docs != c.docs;
        if changed && let Some(c) = workspace.edit(name) {
            c.auth = match auth_ {
                Auth::Inherit => Auth::None,
                other => other,
            };
            c.headers = headers;
            c.variables = variables;
            c.docs = docs;
        }
        open
    }

    fn folder(&mut self, ctx: &egui::Context, workspace: &mut Workspace, id: RequestId) -> bool {
        let mut open = true;
        let Some(c) = workspace.collection_of(id) else {
            return false;
        };
        let Some(f) = c.folder(id) else {
            return false;
        };
        let (mut auth_, mut headers) = (f.auth.clone(), f.headers.clone());
        let inherited = c
            .ancestors(id)
            .iter()
            .rev()
            .find(|a| a.auth != Auth::Inherit)
            .map_or_else(
                || auth::describe(&c.auth, &format!("collection {}", c.name)),
                |a| auth::describe(&a.auth, &format!("folder {}", a.name)),
            );
        let vars = workspace.variables_of_collection(&c.name);
        let title = format!("{} {} — settings", icon::FOLDER, f.name);
        egui::Window::new(title)
            .id(egui::Id::new(("folder settings", id)))
            .open(&mut open)
            .default_size([620.0, 380.0])
            .collapsible(false)
            .show(ctx, |ui| {
                self.tabs(ui, &["Auth", "Headers"]);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.tab == 0 {
                        hint(ui, "Used by the requests of this folder set to inherit.");
                        auth::editor(ui, &mut auth_, Some(&inherited), &vars);
                    } else {
                        hint(
                            ui,
                            "Sent with this folder's requests, after the collection's.",
                        );
                        key_values(
                            ui,
                            "f headers",
                            &mut headers,
                            &vars,
                            Columns::new("Header", "Value"),
                        );
                    }
                });
            });
        if (auth_ != f.auth || headers != f.headers)
            && let Some(f) = workspace.edit_folder(id)
        {
            f.auth = auth_;
            f.headers = headers;
        }
        open
    }

    fn tabs(&mut self, ui: &mut Ui, labels: &[&str]) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for (i, label) in labels.iter().enumerate() {
                if theme::pill(ui, self.tab == i, *label).clicked() {
                    self.tab = i;
                }
            }
        });
        ui.separator();
    }
}

#[derive(Debug)]
pub enum SettingsOutcome {
    Open,
    Closed,
    Save(Settings),
    ClearCookies,
}

/// The settings, edited on a copy until Save.
#[derive(Debug)]
pub struct SettingsWindow {
    draft: Settings,
    proxy: String,
}

impl SettingsWindow {
    pub fn new(current: &Settings) -> Self {
        Self {
            draft: current.clone(),
            proxy: current.proxy.clone().unwrap_or_default(),
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, cookies: &[Cookie]) -> SettingsOutcome {
        let mut open = true;
        let mut outcome = SettingsOutcome::Open;
        egui::Window::new(format!("{} Settings", icon::GEAR_SIX))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(460.0)
            .show(ctx, |ui| {
                heading(ui, "REQUESTS");
                egui::Grid::new("settings")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Timeout");
                        ui.add(
                            egui::DragValue::new(&mut self.draft.timeout_secs)
                                .range(1..=3600)
                                .suffix(" s"),
                        );
                        ui.end_row();
                        ui.label("Redirects");
                        ui.checkbox(&mut self.draft.follow_redirects, "Follow them");
                        ui.end_row();
                        ui.label("Certificates");
                        ui.checkbox(&mut self.draft.verify_tls, "Check them");
                        ui.end_row();
                        ui.label("Proxy");
                        ui.add(
                            TextEdit::singleline(&mut self.proxy)
                                .hint_text("http://proxy:3128 (empty: HTTPS_PROXY / HTTP_PROXY)")
                                .desired_width(280.0),
                        );
                        ui.end_row();
                        ui.label("Cookies");
                        ui.checkbox(&mut self.draft.cookies, "Keep them and send them back");
                        ui.end_row();
                        ui.label("Theme");
                        egui::widgets::global_theme_preference_buttons(ui);
                        ui.end_row();
                    });
                if !self.draft.verify_tls {
                    ui.label(
                        RichText::new(format!(
                            "{} Any certificate is accepted: only for your own servers.",
                            icon::WARNING
                        ))
                        .color(theme::error(ui)),
                    );
                }
                heading(ui, &format!("COOKIES KEPT ({})", cookies.len()));
                if cookies.is_empty() {
                    hint(ui, "None yet.");
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(140.0)
                        .show(ui, |ui| {
                            for c in cookies {
                                ui.label(
                                    RichText::new(format!("{} = {}", c.name, c.value))
                                        .monospace()
                                        .small(),
                                );
                            }
                        });
                    if ui.button(format!("{} Forget them", icon::BROOM)).clicked() {
                        outcome = SettingsOutcome::ClearCookies;
                    }
                }
                ui.add_space(10.0);
                if ui.button(format!("{} Save", icon::CHECK)).clicked() {
                    let proxy = self.proxy.trim();
                    self.draft.proxy = (!proxy.is_empty()).then(|| proxy.to_owned());
                    outcome = SettingsOutcome::Save(self.draft.clone());
                }
            });
        if !open {
            return SettingsOutcome::Closed;
        }
        outcome
    }
}

/// A request as a curl command, to copy.
#[derive(Debug)]
pub struct CodeWindow {
    pub command: String,
}

impl CodeWindow {
    /// `false` once closed.
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;
        egui::Window::new(format!("{} cURL", icon::CODE))
            .open(&mut open)
            .default_size([620.0, 300.0])
            .collapsible(false)
            .show(ctx, |ui| {
                if ui.button(format!("{} Copy", icon::COPY)).clicked() {
                    ui.ctx().copy_text(self.command.clone());
                }
                egui::ScrollArea::both().show(ui, |ui| {
                    let mut text = self.command.as_str();
                    ui.add(
                        TextEdit::multiline(&mut text)
                            .code_editor()
                            .desired_width(f32::INFINITY),
                    );
                });
            });
        open
    }
}
