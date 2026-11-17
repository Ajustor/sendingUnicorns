//! The side panel: collections (folders, drag and drop) and the history.

use std::collections::HashSet;

use domain::{Collection, CollectionName, History, Item, Place, Request, RequestId, Workspace};
use egui::{CornerRadius, RichText, ScrollArea, Sense, TextEdit, Ui};
use egui_phosphor::regular as icon;

use super::command::Command;
use super::theme;

const ROW_HEIGHT: f32 = 26.0;
const INDENT: f32 = 14.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum View {
    #[default]
    Collections,
    History,
}

/// What is being dragged in the tree.
#[derive(Debug, Clone, Copy)]
struct Dragged(RequestId);

#[derive(Debug, Default)]
pub struct Sidebar {
    view: View,
    filter: String,
    /// Folders and collections folded by the user (all open otherwise).
    closed: HashSet<String>,
}

/// Where a row is: to draw it indented and to place what is dropped on it.
#[derive(Debug, Clone, Copy)]
struct Row<'a> {
    collection: &'a CollectionName,
    /// The folder holding the row (`None` at the collection's root).
    parent: Option<RequestId>,
    index: usize,
    depth: usize,
}

impl Sidebar {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        workspace: &Workspace,
        history: &History,
        active: Option<RequestId>,
        commands: &mut Vec<Command>,
    ) {
        egui::SidePanel::left("side panel")
            .resizable(true)
            .default_width(280.0)
            .width_range(200.0..=520.0)
            .frame(
                egui::Frame::NONE
                    .fill(theme::side_fill(ctx.style().visuals.dark_mode))
                    .inner_margin(egui::Margin::same(10)),
            )
            .show(ctx, |ui| {
                self.header(ui, commands);
                ui.add_space(4.0);
                ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| match self.view {
                        View::Collections => self.collections(ui, workspace, active, commands),
                        View::History => self.history(ui, history, commands),
                    });
            });
    }

    fn header(&mut self, ui: &mut Ui, commands: &mut Vec<Command>) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let collections = format!("{} Collections", icon::FOLDERS);
            if theme::pill(ui, self.view == View::Collections, collections).clicked() {
                self.view = View::Collections;
            }
            let history = format!("{} History", icon::CLOCK_COUNTER_CLOCKWISE);
            if theme::pill(ui, self.view == View::History, history).clicked() {
                self.view = View::History;
            }
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add(
                TextEdit::singleline(&mut self.filter)
                    .hint_text(format!("{} Filter", icon::MAGNIFYING_GLASS))
                    .desired_width(ui.available_width() - 92.0),
            );
            match self.view {
                View::Collections => {
                    if theme::icon_button(ui, icon::PLUS, "New collection").clicked() {
                        commands.push(Command::NewCollection);
                    }
                    if theme::icon_button(ui, icon::TERMINAL_WINDOW, "Import a cURL command")
                        .clicked()
                    {
                        commands.push(Command::ImportCurl);
                    }
                    if theme::icon_button(
                        ui,
                        icon::DOWNLOAD_SIMPLE,
                        "Import a collection (ours, Postman)",
                    )
                    .clicked()
                    {
                        commands.push(Command::Import);
                    }
                }
                View::History => {
                    if theme::icon_button(ui, icon::BROOM, "Clear the history").clicked() {
                        commands.push(Command::ClearHistory);
                    }
                }
            }
        });
    }

    fn collections(
        &mut self,
        ui: &mut Ui,
        workspace: &Workspace,
        active: Option<RequestId>,
        commands: &mut Vec<Command>,
    ) {
        if workspace.collections().is_empty() {
            empty(ui, commands);
            return;
        }
        let filter = self.filter.trim().to_lowercase();
        for collection in workspace.collections() {
            if !filter.is_empty() && !matches_collection(collection, &filter) {
                continue;
            }
            self.collection_row(
                ui,
                collection,
                workspace.is_dirty(&collection.name),
                commands,
            );
            if self.is_open(collection.name.as_str()) || !filter.is_empty() {
                let row = Row {
                    collection: &collection.name,
                    parent: None,
                    index: 0,
                    depth: 1,
                };
                self.items(ui, &collection.items, row, &filter, active, commands);
                if collection.items.is_empty() {
                    ui.horizontal(|ui| {
                        ui.add_space(INDENT + 8.0);
                        let add = ui.add(
                            egui::Button::new(
                                RichText::new(format!("{} Add a request", icon::PLUS))
                                    .color(theme::weak(ui)),
                            )
                            .frame(false),
                        );
                        if add.clicked() {
                            commands.push(Command::NewRequest {
                                collection: collection.name.clone(),
                                folder: None,
                            });
                        }
                    });
                }
            }
            ui.add_space(2.0);
        }
    }

    fn is_open(&self, key: &str) -> bool {
        !self.closed.contains(key)
    }

    fn toggle(&mut self, key: String) {
        if !self.closed.remove(&key) {
            self.closed.insert(key);
        }
    }

    fn collection_row(
        &mut self,
        ui: &mut Ui,
        c: &Collection,
        dirty: bool,
        commands: &mut Vec<Command>,
    ) {
        let open = self.is_open(c.name.as_str());
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), ROW_HEIGHT + 2.0),
            Sense::click(),
        );
        if response.hovered() || response.dnd_hover_payload::<Dragged>().is_some() {
            ui.painter().rect_filled(
                rect,
                CornerRadius::same(6),
                ui.visuals().widgets.hovered.weak_bg_fill,
            );
        }
        let painter = ui.painter();
        let y = rect.center().y;
        let caret = if open {
            icon::CARET_DOWN
        } else {
            icon::CARET_RIGHT
        };
        painter.text(
            egui::pos2(rect.left() + 4.0, y),
            egui::Align2::LEFT_CENTER,
            caret,
            egui::FontId::proportional(12.0),
            theme::weak(ui),
        );
        painter.text(
            egui::pos2(rect.left() + 20.0, y),
            egui::Align2::LEFT_CENTER,
            icon::FOLDERS,
            egui::FontId::proportional(15.0),
            theme::accent(ui),
        );
        let name = painter.text(
            egui::pos2(rect.left() + 40.0, y),
            egui::Align2::LEFT_CENTER,
            c.name.as_str(),
            egui::FontId::proportional(14.5),
            ui.visuals().strong_text_color(),
        );
        if dirty {
            painter.circle_filled(egui::pos2(name.right() + 8.0, y), 3.5, theme::PINK);
        }
        if response.clicked() {
            self.toggle(c.name.to_string());
        }
        if let Some(dragged) = response.dnd_release_payload::<Dragged>() {
            commands.push(Command::Move(
                dragged.0,
                Place {
                    collection: c.name.clone(),
                    folder: None,
                    index: None,
                },
            ));
        }
        let name = c.name.clone();
        response
            .on_hover_text(format!("{} requests", c.requests().len()))
            .context_menu(|ui| {
                let items = collection_menu(&name);
                menu(ui, items, commands);
            });
    }

    fn items(
        &mut self,
        ui: &mut Ui,
        items: &[Item],
        place: Row<'_>,
        filter: &str,
        active: Option<RequestId>,
        commands: &mut Vec<Command>,
    ) {
        for (index, item) in items.iter().enumerate() {
            let row = Row { index, ..place };
            match item {
                Item::Request(r) => {
                    if filter.is_empty() || matches_request(r, filter) {
                        request_row(ui, r, row, active == Some(r.id), commands);
                    }
                }
                Item::Folder(f) => {
                    let key = format!("folder {:?}", f.id);
                    let visible = filter.is_empty()
                        || f.name.to_lowercase().contains(filter)
                        || folder_matches(&f.items, filter);
                    if !visible {
                        continue;
                    }
                    let open = self.is_open(&key) || !filter.is_empty();
                    if folder_row(ui, f, row, open, commands) {
                        self.toggle(key);
                    }
                    if open {
                        let inner = Row {
                            parent: Some(f.id),
                            index: 0,
                            depth: row.depth + 1,
                            ..row
                        };
                        self.items(ui, &f.items, inner, filter, active, commands);
                    }
                }
            }
        }
    }

    fn history(&self, ui: &mut Ui, history: &History, commands: &mut Vec<Command>) {
        if history.is_empty() {
            ui.add_space(12.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Nothing sent yet.").color(theme::weak(ui)));
            });
            return;
        }
        let filter = self.filter.trim().to_lowercase();
        let now = updater::unix_now();
        for (index, entry) in history.entries().enumerate() {
            let r = &entry.request;
            if !filter.is_empty() && !matches_request(r, &filter) {
                continue;
            }
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), ROW_HEIGHT + 14.0),
                Sense::click(),
            );
            if response.hovered() {
                ui.painter().rect_filled(
                    rect,
                    CornerRadius::same(6),
                    ui.visuals().widgets.hovered.weak_bg_fill,
                );
            }
            let painter = ui.painter_at(rect);
            let top = rect.top() + 11.0;
            painter.text(
                egui::pos2(rect.left() + 6.0, top),
                egui::Align2::LEFT_CENTER,
                r.method.as_str(),
                egui::FontId::monospace(10.5),
                theme::method(ui, r.method),
            );
            painter.text(
                egui::pos2(rect.left() + 58.0, top),
                egui::Align2::LEFT_CENTER,
                &r.name,
                egui::FontId::proportional(13.5),
                ui.visuals().text_color(),
            );
            let status = entry
                .status
                .map_or_else(|| "failed".to_owned(), |s| s.to_string());
            let detail = format!(
                "{status} · {} ms · {}",
                entry.elapsed_ms,
                ago(now.saturating_sub(entry.at))
            );
            painter.text(
                egui::pos2(rect.left() + 58.0, rect.bottom() - 9.0),
                egui::Align2::LEFT_CENTER,
                detail,
                egui::FontId::proportional(11.0),
                theme::weak(ui),
            );
            if response.on_hover_text(&r.url).clicked() {
                commands.push(Command::OpenHistory(index));
            }
        }
    }
}

#[allow(clippy::cast_precision_loss)] // a few levels
fn indent(depth: usize) -> f32 {
    INDENT * depth as f32
}

/// A request: method, name; dragged to move it, dropped on to put what is
/// dragged before or after it.
fn request_row(ui: &mut Ui, r: &Request, row: Row<'_>, active: bool, commands: &mut Vec<Command>) {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ROW_HEIGHT),
        Sense::click_and_drag(),
    );
    response.dnd_set_drag_payload(Dragged(r.id));
    let left = rect.left() + indent(row.depth);
    if active {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), ui.visuals().selection.bg_fill);
    } else if response.hovered() {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(6),
            ui.visuals().widgets.hovered.weak_bg_fill,
        );
    }
    let painter = ui.painter_at(rect);
    let y = rect.center().y;
    painter.text(
        egui::pos2(left, y),
        egui::Align2::LEFT_CENTER,
        r.method.as_str(),
        egui::FontId::monospace(10.5),
        theme::method(ui, r.method),
    );
    let mut job = egui::text::LayoutJob::simple_singleline(
        r.name.clone(),
        egui::FontId::proportional(13.5),
        ui.visuals().text_color(),
    );
    job.wrap = egui::text::TextWrapping::truncate_at_width(rect.right() - left - 56.0);
    let name = painter.layout_job(job);
    painter.galley(
        egui::pos2(left + 52.0, y - name.size().y / 2.0),
        name,
        ui.visuals().text_color(),
    );

    drop_between(ui, &response, rect, row, r.id, commands);
    if response.clicked() {
        commands.push(Command::Open(r.id));
    }
    let id = r.id;
    let collection = row.collection.clone();
    let folder = row.parent;
    response.on_hover_text(&r.url).context_menu(|ui| {
        let items = [
            (icon::PENCIL_SIMPLE, "Rename", Command::Rename(id)),
            (icon::COPY, "Duplicate", Command::DuplicateRequest(id)),
            (
                icon::PLUS,
                "New request here",
                Command::NewRequest {
                    collection: collection.clone(),
                    folder,
                },
            ),
            (icon::TRASH, "Delete", Command::Delete(id)),
        ];
        menu(ui, items, commands);
    });
}

/// A folder; `true` when clicked (to fold or unfold it).
fn folder_row(
    ui: &mut Ui,
    f: &domain::Folder,
    row: Row<'_>,
    open: bool,
    commands: &mut Vec<Command>,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ROW_HEIGHT),
        Sense::click_and_drag(),
    );
    response.dnd_set_drag_payload(Dragged(f.id));
    let left = rect.left() + indent(row.depth) - 12.0;
    let below_top = |p: egui::Pos2| p.y > rect.top() + rect.height() * 0.3;
    let into = response
        .dnd_hover_payload::<Dragged>()
        .is_some_and(|d| d.0 != f.id)
        && response.hover_pos().is_some_and(below_top);
    if into {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(6),
            egui::Stroke::new(1.5_f32, theme::accent(ui)),
            egui::StrokeKind::Inside,
        );
    } else if response.hovered() {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(6),
            ui.visuals().widgets.hovered.weak_bg_fill,
        );
    }
    let painter = ui.painter_at(rect);
    let y = rect.center().y;
    let (caret, folder) = if open {
        (icon::CARET_DOWN, icon::FOLDER_OPEN)
    } else {
        (icon::CARET_RIGHT, icon::FOLDER)
    };
    painter.text(
        egui::pos2(left, y),
        egui::Align2::LEFT_CENTER,
        caret,
        egui::FontId::proportional(11.0),
        theme::weak(ui),
    );
    painter.text(
        egui::pos2(left + 14.0, y),
        egui::Align2::LEFT_CENTER,
        folder,
        egui::FontId::proportional(14.0),
        theme::accent(ui).gamma_multiply(0.85),
    );
    painter.text(
        egui::pos2(left + 34.0, y),
        egui::Align2::LEFT_CENTER,
        &f.name,
        egui::FontId::proportional(13.5),
        ui.visuals().text_color(),
    );
    if let Some(dragged) = response.dnd_release_payload::<Dragged>() {
        let place = if response.hover_pos().is_some_and(below_top) {
            Place {
                collection: row.collection.clone(),
                folder: Some(f.id),
                index: None,
            }
        } else {
            Place {
                collection: row.collection.clone(),
                folder: row.parent,
                index: Some(row.index),
            }
        };
        if dragged.0 != f.id {
            commands.push(Command::Move(dragged.0, place));
        }
    } else if !into {
        // Dropping over its top edge: before it.
        show_drop_line(ui, &response, rect, true);
    }
    let id = f.id;
    let collection = row.collection.clone();
    let clicked = response.clicked();
    response.context_menu(|ui| {
        let items = folder_menu(&collection, id);
        menu(ui, items, commands);
    });
    clicked
}

fn collection_menu(name: &CollectionName) -> [(&'static str, &'static str, Command); 8] {
    [
        (
            icon::PLUS,
            "New request",
            Command::NewRequest {
                collection: name.clone(),
                folder: None,
            },
        ),
        (
            icon::FOLDER_PLUS,
            "New folder",
            Command::NewFolder {
                collection: name.clone(),
                parent: None,
            },
        ),
        (
            icon::PLAY,
            "Run the collection",
            Command::Run {
                collection: name.clone(),
                folder: None,
            },
        ),
        (
            icon::GLOBE,
            "Environments…",
            Command::ManageEnvironments(name.clone()),
        ),
        (
            icon::GEAR_SIX,
            "Settings: auth, headers, variables…",
            Command::CollectionSettings(name.clone()),
        ),
        (
            icon::PENCIL_SIMPLE,
            "Rename",
            Command::RenameCollection(name.clone()),
        ),
        (icon::EXPORT, "Export…", Command::Export(name.clone())),
        (
            icon::TRASH,
            "Delete",
            Command::DeleteCollection(name.clone()),
        ),
    ]
}

fn folder_menu(
    collection: &CollectionName,
    id: RequestId,
) -> [(&'static str, &'static str, Command); 6] {
    [
        (
            icon::PLUS,
            "New request",
            Command::NewRequest {
                collection: collection.clone(),
                folder: Some(id),
            },
        ),
        (
            icon::FOLDER_PLUS,
            "New folder",
            Command::NewFolder {
                collection: collection.clone(),
                parent: Some(id),
            },
        ),
        (
            icon::PLAY,
            "Run the folder",
            Command::Run {
                collection: collection.clone(),
                folder: Some(id),
            },
        ),
        (
            icon::GEAR_SIX,
            "Settings: auth, headers…",
            Command::FolderSettings(id),
        ),
        (icon::PENCIL_SIMPLE, "Rename", Command::Rename(id)),
        (icon::TRASH, "Delete", Command::Delete(id)),
    ]
}

/// Something dropped on a request: before it (top half) or after it.
fn drop_between(
    ui: &Ui,
    response: &egui::Response,
    rect: egui::Rect,
    row: Row<'_>,
    target: RequestId,
    commands: &mut Vec<Command>,
) {
    let Some(pointer) = response.hover_pos() else {
        return;
    };
    let before = pointer.y < rect.center().y;
    if let Some(dragged) = response.dnd_release_payload::<Dragged>() {
        if dragged.0 != target {
            let index = if before { row.index } else { row.index + 1 };
            commands.push(Command::Move(
                dragged.0,
                Place {
                    collection: row.collection.clone(),
                    folder: row.parent,
                    index: Some(index),
                },
            ));
        }
        return;
    }
    show_drop_line(ui, response, rect, before);
}

fn show_drop_line(ui: &Ui, response: &egui::Response, rect: egui::Rect, before: bool) {
    if response.dnd_hover_payload::<Dragged>().is_some() {
        let y = if before { rect.top() } else { rect.bottom() };
        ui.painter()
            .hline(rect.x_range(), y, egui::Stroke::new(2.0_f32, theme::CYAN));
    }
}

fn menu<const N: usize>(
    ui: &mut Ui,
    items: [(&str, &str, Command); N],
    commands: &mut Vec<Command>,
) {
    for (glyph, label, command) in items {
        if ui.button(format!("{glyph}  {label}")).clicked() {
            commands.push(command);
            ui.close_menu();
        }
    }
}

fn matches_request(r: &Request, filter: &str) -> bool {
    r.name.to_lowercase().contains(filter) || r.url.to_lowercase().contains(filter)
}

fn folder_matches(items: &[Item], filter: &str) -> bool {
    items.iter().any(|item| match item {
        Item::Request(r) => matches_request(r, filter),
        Item::Folder(f) => {
            f.name.to_lowercase().contains(filter) || folder_matches(&f.items, filter)
        }
    })
}

fn matches_collection(c: &Collection, filter: &str) -> bool {
    c.name.as_str().to_lowercase().contains(filter) || folder_matches(&c.items, filter)
}

/// `12 s ago`, `5 min ago`, `3 h ago`, `2 d ago`.
fn ago(seconds: u64) -> String {
    match seconds {
        0..60 => format!("{seconds} s ago"),
        60..3600 => format!("{} min ago", seconds / 60),
        3600..86_400 => format!("{} h ago", seconds / 3600),
        _ => format!("{} d ago", seconds / 86_400),
    }
}

fn empty(ui: &mut Ui, commands: &mut Vec<Command>) {
    ui.add_space(16.0);
    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new(icon::FOLDERS)
                .size(36.0)
                .color(theme::accent(ui).gamma_multiply(0.6)),
        );
        ui.label(RichText::new("No collection yet").color(theme::weak(ui)));
        ui.add_space(8.0);
        let big = |text: String| egui::Button::new(text).min_size(egui::vec2(220.0, 30.0));
        if ui
            .add(big(format!("{} New collection", icon::PLUS)))
            .clicked()
        {
            commands.push(Command::NewCollection);
        }
        if ui
            .add(big(format!(
                "{} Import (ours, Postman)…",
                icon::DOWNLOAD_SIMPLE
            )))
            .clicked()
        {
            commands.push(Command::Import);
        }
        if ui
            .add(big(format!(
                "{} Paste a cURL command…",
                icon::TERMINAL_WINDOW
            )))
            .clicked()
        {
            commands.push(Command::ImportCurl);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ages() {
        assert_eq!(ago(5), "5 s ago");
        assert_eq!(ago(125), "2 min ago");
        assert_eq!(ago(7200), "2 h ago");
        assert_eq!(ago(200_000), "2 d ago");
    }
}
