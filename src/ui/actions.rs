//! What each [`Command`] does, and the dialogs' and windows' outcomes.

use domain::{CollectionName, Item, Place, Request, RequestId};
use egui::ViewportCommand;

use super::app::SendingApp;
use super::command::Command;
use super::dialogs::{
    CodeWindow, DeleteTarget, Dialog, EnvironmentAction, EnvironmentsWindow, ItemSettings,
    ItemSettingsWindow, NamePurpose, Outcome, SettingsOutcome, SettingsWindow,
};
use super::response::Exchange;
use super::runner::RunnerWindow;

impl SendingApp {
    pub(super) fn run(&mut self, ctx: &egui::Context, command: Command) {
        match command {
            Command::Open(id) => self.open_tab(id),
            Command::CloseTab(id) => self.close_tab(id),
            Command::CloseActiveTab => {
                if let Some(id) = self.active {
                    self.close_tab(id);
                }
            }
            Command::CycleTabs(step) => self.cycle_tabs(step),
            Command::MoveTab(id, to) => {
                if let Some(from) = self.open.iter().position(|&t| t == id) {
                    let tab = self.open.remove(from);
                    self.open.insert(to.min(self.open.len()), tab);
                }
            }
            Command::NewTab => {
                let id = self.workspace.add_draft(Request::new("Untitled request"));
                self.open_tab(id);
            }
            Command::Send => self.send(),
            Command::CancelSend => self.sender.cancel(),
            Command::Save => self.save(),
            Command::Import => self.import(),
            Command::ImportCurl => self.import_curl(),
            Command::CopyAsCurl => self.copy_as_curl(),
            Command::Export(name) => self.export(&name),
            Command::DuplicateRequest(id) => {
                let collection = self.workspace.collection_of(id).map(|c| c.name.clone());
                if let Some(copy) = collection
                    .and_then(|name| self.workspace.edit(&name))
                    .and_then(|c| c.duplicate_request(id))
                {
                    self.open_tab(copy);
                }
            }
            Command::Move(id, place) => {
                if !self.workspace.move_to(id, &place) {
                    self.error("It can't go there.");
                }
            }
            Command::SetEnvironment(collection, env) => {
                self.workspace.set_active_environment(&collection, env);
            }
            Command::Run { collection, folder } => self.start_run(ctx, collection, folder),
            Command::OpenHistory(index) => {
                if let Some(entry) = self.history.get(index) {
                    let request = entry.request.duplicate(entry.request.name.clone());
                    let id = self.workspace.add_draft(request);
                    self.open_tab(id);
                }
            }
            Command::ClearHistory => {
                if let Err(e) = self.use_cases.history.clear(&mut self.history) {
                    self.error(format!("History: {e}"));
                }
            }
            Command::SaveResponseBody => self.save_response_body(),
            Command::CheckForUpdates => self.updates.check_now(),
            Command::InstallUpdate => self.updates.updater.install(),
            Command::UpdateLater => self.updates.updater.later(),
            Command::SkipUpdate => self.updates.skip(),
            Command::RestartToUpdate => {
                self.updates.updater.schedule_restart();
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            other => self.ask(other),
        }
    }

    /// Commands that open a dialog or a window.
    fn ask(&mut self, command: Command) {
        match command {
            Command::NewCollection => {
                self.dialog = Some(Dialog::name(NamePurpose::NewCollection, ""));
            }
            Command::RenameCollection(name) => {
                self.dialog = Some(Dialog::name(
                    NamePurpose::RenameCollection(name.clone()),
                    name.as_str(),
                ));
            }
            Command::DeleteCollection(name) => {
                self.dialog = Some(Dialog::Delete {
                    what: format!("the collection {name:?}"),
                    target: DeleteTarget::Collection(name),
                });
            }
            Command::CollectionSettings(name) => {
                self.item_settings = Some(ItemSettingsWindow::new(ItemSettings::Collection(name)));
            }
            Command::FolderSettings(id) => {
                self.item_settings = Some(ItemSettingsWindow::new(ItemSettings::Folder(id)));
            }
            Command::NewRequest { collection, folder } => {
                self.dialog = Some(Dialog::name(
                    NamePurpose::NewRequest { collection, folder },
                    "New request",
                ));
            }
            Command::NewFolder { collection, parent } => {
                self.dialog = Some(Dialog::name(
                    NamePurpose::NewFolder { collection, parent },
                    "New folder",
                ));
            }
            Command::Rename(id) => {
                if let Some(name) = self.item_name(id) {
                    self.dialog = Some(Dialog::name(NamePurpose::Rename(id), name));
                }
            }
            Command::Delete(id) => {
                if let Some(name) = self.item_name(id) {
                    self.dialog = Some(Dialog::Delete {
                        what: format!("{name:?}"),
                        target: DeleteTarget::Item(id),
                    });
                }
            }
            Command::ManageEnvironments(collection) => {
                let active = self
                    .workspace
                    .active_environment(&collection)
                    .map(|e| e.name.clone());
                self.environments = Some(EnvironmentsWindow::new(collection, active));
            }
            Command::ManageGlobals => self.environments = Some(EnvironmentsWindow::globals()),
            Command::OpenSettings => {
                self.settings_window = Some(SettingsWindow::new(&self.settings));
            }
            _ => {}
        }
    }

    fn open_tab(&mut self, id: RequestId) {
        if !self.open.contains(&id) {
            let at = self
                .active
                .and_then(|a| self.open.iter().position(|&t| t == a))
                .map_or(self.open.len(), |i| i + 1);
            self.open.insert(at, id);
        }
        self.active = Some(id);
    }

    fn close_tab(&mut self, id: RequestId) {
        let draft = self
            .workspace
            .find(id)
            .filter(|f| f.collection.is_none())
            .map(|f| f.request.clone());
        if let Some(draft) = draft {
            let blank = Request {
                id: draft.id,
                name: draft.name.clone(),
                ..Request::new("")
            };
            if draft != blank {
                self.dialog = Some(Dialog::CloseDraft(id));
                return;
            }
            self.workspace.remove(id);
        }
        self.drop_tab(id);
    }

    fn drop_tab(&mut self, id: RequestId) {
        let Some(index) = self.open.iter().position(|&t| t == id) else {
            return;
        };
        self.open.remove(index);
        self.exchanges.remove(&id);
        if self.active == Some(id) {
            self.active = self.open.get(index).or_else(|| self.open.last()).copied();
        }
    }

    fn cycle_tabs(&mut self, step: isize) {
        let Some(current) = self
            .active
            .and_then(|a| self.open.iter().position(|&t| t == a))
        else {
            return;
        };
        let len = self.open.len().cast_signed();
        let next = (current.cast_signed() + step).rem_euclid(len);
        self.active = self.open.get(next.cast_unsigned()).copied();
    }

    fn item_name(&self, id: RequestId) -> Option<String> {
        if let Some(found) = self.workspace.find(id) {
            return Some(found.request.name.clone());
        }
        let c = self.workspace.collection_of(id)?;
        c.folder(id).map(|f| f.name.clone())
    }

    fn send(&mut self) {
        let Some(id) = self.active else {
            return;
        };
        if self.sender.is_busy() {
            return;
        }
        match application::sending::SendRequest::prepare(&self.workspace, id) {
            Ok(prepared) => {
                self.sender.start(
                    self.use_cases.send.clone(),
                    id,
                    prepared,
                    self.settings.clone(),
                );
            }
            Err(e) => {
                let exchange = Exchange::new(
                    String::new(),
                    Err(e.to_string()),
                    std::collections::BTreeSet::new(),
                    domain::AfterResponse::default(),
                );
                self.exchanges.insert(id, exchange);
            }
        }
    }

    fn save(&mut self) {
        if let Some(id) = self.active
            && self.workspace.is_draft(id)
        {
            let name = self.item_name(id).unwrap_or_default();
            let collection = self.workspace.collections().first().map(|c| c.name.clone());
            self.dialog = Some(Dialog::SaveDraft {
                id,
                name,
                collection,
                error: None,
            });
            return;
        }
        if !self.workspace.has_unsaved_changes() {
            self.info("Nothing to save");
            return;
        }
        let failures = self.use_cases.save.execute(&mut self.workspace);
        match failures.first() {
            None => self.info(format!("{} Saved", egui_phosphor::regular::CHECK)),
            Some(e) => self.error(format!("Not saved: {e}")),
        }
    }

    fn import(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Import a collection")
            .add_filter("Collection (sendingUnicorns, Postman)", &["json"])
            .pick_file()
        else {
            return;
        };
        match self.use_cases.import.execute(&mut self.workspace, &path) {
            Ok(name) => {
                let first = self
                    .workspace
                    .collection(&name)
                    .and_then(|c| c.requests().first().map(|r| r.id));
                if let Some(id) = first {
                    self.open_tab(id);
                }
                self.info(format!("Imported {name:?}"));
            }
            Err(e) => self.error(format!("Import: {e}")),
        }
    }

    /// A curl command typed in the address bar becomes the request; else
    /// the paste dialog opens.
    fn import_curl(&mut self) {
        let typed = self
            .active
            .and_then(|id| self.workspace.find(id))
            .filter(|f| f.request.url.trim_start().starts_with("curl "))
            .map(|f| (f.request.id, f.request.name.clone(), f.request.url.clone()));
        let Some((id, name, command)) = typed else {
            self.dialog = Some(Dialog::Curl {
                text: String::new(),
                error: None,
            });
            return;
        };
        match domain::parse_curl(&command) {
            Ok(parsed) => {
                if let Some(request) = self.workspace.edit_request(id) {
                    let keep_name = name != "Untitled request";
                    *request = Request {
                        id,
                        name: if keep_name { name } else { parsed.name.clone() },
                        ..parsed
                    };
                }
            }
            // Still being typed: wait for the rest.
            Err(domain::CurlError::Unclosed | domain::CurlError::NoUrl) => {}
            Err(e) => self.error(format!("cURL: {e}")),
        }
    }

    fn copy_as_curl(&mut self) {
        let Some(id) = self.active else {
            return;
        };
        match application::sending::SendRequest::prepare(&self.workspace, id) {
            Ok(prepared) => {
                self.code = Some(CodeWindow {
                    command: domain::to_curl(&prepared),
                });
            }
            Err(e) => self.error(e.to_string()),
        }
    }

    fn export(&mut self, name: &CollectionName) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Export the collection")
            .set_file_name(format!("{name}.json"))
            .add_filter("Collection", &["json"])
            .save_file()
        else {
            return;
        };
        match self.use_cases.export.execute(&self.workspace, name, &path) {
            Ok(()) => self.info(format!("Exported to {}", path.display())),
            Err(e) => self.error(format!("Export: {e}")),
        }
    }

    fn save_response_body(&mut self) {
        let Some(Ok(response)) = self
            .active
            .and_then(|id| self.exchanges.get(&id))
            .map(|e| &e.result)
        else {
            return;
        };
        let extension = match response.content_kind() {
            domain::ContentKind::Json => "json",
            domain::ContentKind::Html => "html",
            domain::ContentKind::Xml => "xml",
            domain::ContentKind::Text => "txt",
            domain::ContentKind::Binary => "bin",
        };
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(format!("response.{extension}"))
            .save_file()
        else {
            return;
        };
        match std::fs::write(&path, &response.body) {
            Ok(()) => self.info(format!("Saved to {}", path.display())),
            Err(e) => self.error(format!("Could not write {}: {e}", path.display())),
        }
    }

    fn start_run(
        &mut self,
        ctx: &egui::Context,
        collection: CollectionName,
        folder: Option<RequestId>,
    ) {
        let title = match folder.and_then(|f| self.item_name(f)) {
            Some(f) => format!("{collection} › {f}"),
            None => collection.to_string(),
        };
        self.runner = Some(RunnerWindow::start(
            self.use_cases.run.clone(),
            self.workspace.clone(),
            collection,
            folder,
            title,
            self.settings.clone(),
            ctx.clone(),
        ));
    }

    fn named(&mut self, purpose: NamePurpose, text: &str) -> Result<(), String> {
        let text = text.trim();
        let non_empty = || {
            if text.is_empty() {
                Err("the name is empty".to_owned())
            } else {
                Ok(())
            }
        };
        match purpose {
            NamePurpose::NewCollection => {
                self.use_cases
                    .create
                    .execute(&mut self.workspace, text)
                    .map_err(|e| e.to_string())?;
            }
            NamePurpose::RenameCollection(from) => {
                self.use_cases
                    .rename
                    .execute(&mut self.workspace, &from, text)
                    .map_err(|e| e.to_string())?;
            }
            NamePurpose::NewRequest { collection, folder } => {
                non_empty()?;
                let request = Request::new(text);
                let id = request.id;
                if let Some(c) = self.workspace.edit(&collection) {
                    c.insert(Item::Request(request), folder, None);
                    self.open_tab(id);
                }
            }
            NamePurpose::NewFolder { collection, parent } => {
                non_empty()?;
                self.workspace.add_folder(&collection, parent, text);
            }
            NamePurpose::Rename(id) => {
                non_empty()?;
                if let Some(request) = self.workspace.edit_request(id) {
                    text.clone_into(&mut request.name);
                } else if let Some(folder) = self.workspace.edit_folder(id) {
                    text.clone_into(&mut folder.name);
                }
            }
            NamePurpose::NewEnvironment(collection) => {
                if let Some(c) = self.workspace.edit(&collection) {
                    c.add_environment(text).map_err(|e| e.to_string())?;
                }
                self.environments =
                    Some(EnvironmentsWindow::new(collection, Some(text.to_owned())));
            }
            NamePurpose::RenameEnvironment(collection, from) => {
                let was_active = self
                    .workspace
                    .active_environment(&collection)
                    .is_some_and(|e| e.name == from);
                if let Some(c) = self.workspace.edit(&collection) {
                    c.rename_environment(&from, text)
                        .map_err(|e| e.to_string())?;
                }
                if was_active {
                    self.workspace
                        .set_active_environment(&collection, Some(text.to_owned()));
                }
                self.environments =
                    Some(EnvironmentsWindow::new(collection, Some(text.to_owned())));
            }
        }
        Ok(())
    }

    fn delete(&mut self, target: DeleteTarget) {
        match target {
            DeleteTarget::Collection(name) => {
                if let Err(e) = self.use_cases.delete.execute(&mut self.workspace, &name) {
                    self.error(format!("Delete: {e}"));
                }
            }
            DeleteTarget::Item(id) => {
                self.workspace.remove(id);
                self.exchanges.remove(&id);
            }
            DeleteTarget::Environment(collection, name) => {
                if let Some(c) = self.workspace.edit(&collection) {
                    c.remove_environment(&name);
                }
            }
        }
    }

    pub(super) fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(dialog) = &mut self.dialog else {
            return;
        };
        match dialog.show(ctx, &self.workspace) {
            Outcome::Open => {}
            Outcome::Cancelled => self.dialog = None,
            Outcome::Name(purpose, text) => match self.named(purpose, &text) {
                Ok(()) => self.dialog = None,
                Err(why) => self.refuse(why),
            },
            Outcome::Delete(target) => {
                self.dialog = None;
                self.delete(target);
            }
            Outcome::SaveAndQuit => {
                self.dialog = None;
                if self.use_cases.save.execute(&mut self.workspace).is_empty() {
                    self.may_close = true;
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                } else {
                    self.error("Not saved: quit cancelled");
                }
            }
            Outcome::Quit => {
                self.dialog = None;
                self.may_close = true;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            Outcome::DropDraft(id) => {
                self.dialog = None;
                self.workspace.remove(id);
                self.drop_tab(id);
            }
            Outcome::Curl(text) => match domain::parse_curl(&text) {
                Ok(request) => {
                    self.dialog = None;
                    let id = self.workspace.add_draft(request);
                    self.open_tab(id);
                }
                Err(e) => self.refuse(e.to_string()),
            },
            Outcome::SaveDraft {
                id,
                name,
                collection,
            } => {
                if name.trim().is_empty() {
                    self.refuse("the name is empty".into());
                    return;
                }
                if let Some(request) = self.workspace.edit_request(id) {
                    name.trim().clone_into(&mut request.name);
                }
                let place = Place {
                    collection,
                    folder: None,
                    index: None,
                };
                if self.workspace.move_to(id, &place) {
                    self.dialog = None;
                    self.save();
                } else {
                    self.refuse("This collection no longer exists.".into());
                }
            }
        }
    }

    fn refuse(&mut self, why: String) {
        if let Some(dialog) = &mut self.dialog {
            dialog.refuse(why);
        }
    }

    pub(super) fn windows(&mut self, ctx: &egui::Context, commands: &mut Vec<Command>) {
        if let Some(window) = &mut self.environments {
            let mut actions = Vec::new();
            if !window.show(ctx, &mut self.workspace, &mut actions) {
                self.environments = None;
            }
            if self.dialog.is_none() {
                self.dialog = actions.into_iter().next().map(|action| match action {
                    EnvironmentAction::Add(c) => Dialog::name(NamePurpose::NewEnvironment(c), ""),
                    EnvironmentAction::Rename(c, name) => {
                        Dialog::name(NamePurpose::RenameEnvironment(c, name.clone()), name)
                    }
                    EnvironmentAction::Delete(c, name) => Dialog::Delete {
                        what: format!("the environment {name:?}"),
                        target: DeleteTarget::Environment(c, name),
                    },
                });
            }
        }
        if let Some(window) = &mut self.item_settings
            && !window.show(ctx, &mut self.workspace)
        {
            self.item_settings = None;
        }
        if let Some(window) = &mut self.code
            && !window.show(ctx)
        {
            self.code = None;
        }
        if let Some(window) = &mut self.runner
            && !window.show(ctx, commands)
        {
            self.runner = None;
        }
        if let Some(window) = &mut self.settings_window {
            let cookies = self.use_cases.cookies.list();
            match window.show(ctx, &cookies) {
                SettingsOutcome::Open => {}
                SettingsOutcome::Closed => self.settings_window = None,
                SettingsOutcome::ClearCookies => {
                    self.use_cases.cookies.clear();
                    self.info("Cookies forgotten");
                }
                SettingsOutcome::Save(settings) => {
                    self.settings_window = None;
                    match self.use_cases.change_settings.execute(settings) {
                        Ok(saved) => {
                            self.settings = saved;
                            self.info("Settings saved");
                        }
                        Err(e) => self.error(format!("Settings: {e}")),
                    }
                }
            }
        }
    }
}
