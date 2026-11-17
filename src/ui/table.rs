//! The editable tables: key / value rows (params, headers, form, variables),
//! multipart fields, assertions. Each ends with an empty row that becomes a
//! new one as soon as something is typed in it.

use domain::{AssertOp, Assertion, KeyValue, KeyValues, Part, Variables};
use egui::{ComboBox, RichText, TextEdit, TextStyle, Ui};
use egui_phosphor::regular as icon;

use super::highlight::template_layouter;
use super::theme;

/// What a key / value table offers.
#[derive(Debug, Clone, Copy)]
pub struct Columns<'a> {
    pub key: &'a str,
    pub value: &'a str,
    /// A lock per row that masks its value (environment variables).
    pub secrets: bool,
    /// Rows can't be added or removed (path params follow the address).
    pub fixed: bool,
}

impl<'a> Columns<'a> {
    pub fn new(key: &'a str, value: &'a str) -> Self {
        Self {
            key,
            value,
            secrets: false,
            fixed: false,
        }
    }
}

/// Widths of the key and value columns, room left for `buttons` icons.
fn widths(ui: &Ui, buttons: f32) -> (f32, f32) {
    let spacing = ui.spacing().item_spacing.x;
    let reserved =
        ui.spacing().interact_size.y * (buttons + 1.0) + spacing * (buttons + 3.0) + 20.0;
    let width = (ui.available_width() - reserved).max(160.0);
    let key = (width * 0.38).floor();
    (key, width - key - spacing)
}

/// Whether anything changed.
pub fn key_values(
    ui: &mut Ui,
    id: &str,
    rows: &mut KeyValues,
    variables: &Variables,
    columns: Columns<'_>,
) -> bool {
    let mut changed = false;
    let mut remove = None;
    let buttons = if columns.secrets { 2.0 } else { 1.0 };
    let (key_width, value_width) = widths(ui, buttons);
    egui::Grid::new(id)
        .num_columns(4)
        .spacing([6.0, 4.0])
        .show(ui, |ui| {
            for (i, row) in rows.0.iter_mut().enumerate() {
                changed |= ui
                    .checkbox(&mut row.enabled, "")
                    .on_hover_text(if row.enabled { "Used" } else { "Not used" })
                    .changed();
                if columns.fixed {
                    ui.add_sized(
                        [key_width, ui.spacing().interact_size.y],
                        egui::Label::new(RichText::new(format!(":{}", row.key)).monospace()),
                    );
                } else {
                    changed |= field(ui, &mut row.key, key_width, columns.key, variables, false);
                }
                changed |= field(
                    ui,
                    &mut row.value,
                    value_width,
                    columns.value,
                    variables,
                    row.secret,
                );
                ui.horizontal(|ui| {
                    if columns.secrets {
                        let (glyph, tip) = if row.secret {
                            (icon::LOCK_SIMPLE, "Secret: shown masked")
                        } else {
                            (icon::LOCK_SIMPLE_OPEN, "Mark as secret")
                        };
                        if theme::icon_button(ui, glyph, tip).clicked() {
                            row.secret = !row.secret;
                            changed = true;
                        }
                    }
                    if !columns.fixed && theme::icon_button(ui, icon::TRASH, "Remove").clicked() {
                        remove = Some(i);
                    }
                });
                ui.end_row();
            }
            if !columns.fixed {
                let mut key = String::new();
                let mut value = String::new();
                ui.add_enabled(false, egui::Checkbox::without_text(&mut true));
                let typed_key = field(ui, &mut key, key_width, columns.key, variables, false);
                let typed_value =
                    field(ui, &mut value, value_width, columns.value, variables, false);
                if typed_key || typed_value {
                    rows.0.push(KeyValue::new(key, value));
                    changed = true;
                }
                ui.end_row();
            }
        });
    if let Some(i) = remove {
        rows.0.remove(i);
        changed = true;
    }
    changed
}

fn field(
    ui: &mut Ui,
    text: &mut String,
    width: f32,
    hint: &str,
    variables: &Variables,
    secret: bool,
) -> bool {
    let mut layouter = template_layouter(variables, TextStyle::Body, false);
    let edit = TextEdit::singleline(text).hint_text(hint).password(secret);
    let edit = if secret {
        edit
    } else {
        edit.layouter(&mut layouter)
    };
    // Sized by hand: a grid cell would otherwise shrink it to its content.
    ui.add_sized([width, ui.spacing().interact_size.y], edit)
        .changed()
}

/// Multipart fields: text, or a file picked with the system's dialog.
pub fn parts(ui: &mut Ui, parts: &mut Vec<Part>, variables: &Variables) -> bool {
    let mut changed = false;
    let mut remove = None;
    let (key_width, value_width) = widths(ui, 2.5);
    egui::Grid::new("multipart")
        .num_columns(4)
        .spacing([6.0, 4.0])
        .show(ui, |ui| {
            for (i, part) in parts.iter_mut().enumerate() {
                changed |= ui.checkbox(&mut part.enabled, "").changed();
                changed |= field(ui, &mut part.key, key_width, "Field", variables, false);
                if part.is_file {
                    let label = if part.value.is_empty() {
                        RichText::new(format!("{} Choose a file…", icon::FILE_ARROW_UP))
                            .color(theme::weak(ui))
                    } else {
                        RichText::new(format!("{} {}", icon::FILE, file_name(&part.value)))
                    };
                    let pick = ui.add_sized(
                        [value_width, ui.spacing().interact_size.y],
                        egui::Button::new(label),
                    );
                    if pick.on_hover_text(&part.value).clicked()
                        && let Some(path) = rfd::FileDialog::new().pick_file()
                    {
                        part.value = path.to_string_lossy().into_owned();
                        changed = true;
                    }
                } else {
                    changed |= field(ui, &mut part.value, value_width, "Value", variables, false);
                }
                ui.horizontal(|ui| {
                    let (glyph, tip) = if part.is_file {
                        (icon::TEXT_T, "Make it a text field")
                    } else {
                        (icon::PAPERCLIP, "Make it a file")
                    };
                    if theme::icon_button(ui, glyph, tip).clicked() {
                        part.is_file = !part.is_file;
                        part.value.clear();
                        changed = true;
                    }
                    if theme::icon_button(ui, icon::TRASH, "Remove").clicked() {
                        remove = Some(i);
                    }
                });
                ui.end_row();
            }
        });
    if let Some(i) = remove {
        parts.remove(i);
        changed = true;
    }
    ui.horizontal(|ui| {
        for (is_file, label) in [
            (false, format!("{} Text field", icon::PLUS)),
            (true, format!("{} File", icon::PAPERCLIP)),
        ] {
            if ui.button(label).clicked() {
                parts.push(Part {
                    key: String::new(),
                    value: String::new(),
                    is_file,
                    enabled: true,
                });
                changed = true;
            }
        }
    });
    changed
}

/// The last part of a path.
pub fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// `res.status  eq  200`, one row per assertion.
pub fn assertions(ui: &mut Ui, assertions: &mut Vec<Assertion>, variables: &Variables) -> bool {
    let mut changed = false;
    let mut remove = None;
    let (expr_width, value_width) = widths(ui, 4.5);
    egui::Grid::new("assertions")
        .num_columns(5)
        .spacing([6.0, 4.0])
        .show(ui, |ui| {
            for (i, a) in assertions.iter_mut().enumerate() {
                changed |= ui.checkbox(&mut a.enabled, "").changed();
                changed |= field(ui, &mut a.expr, expr_width, "res.status", variables, false);
                changed |= op_picker(ui, i, &mut a.op);
                if a.op.takes_value() {
                    changed |= field(
                        ui,
                        &mut a.value,
                        (value_width - 110.0).max(80.0),
                        "200",
                        variables,
                        false,
                    );
                } else {
                    ui.label("");
                }
                if theme::icon_button(ui, icon::TRASH, "Remove").clicked() {
                    remove = Some(i);
                }
                ui.end_row();
            }
            let mut expr = String::new();
            ui.add_enabled(false, egui::Checkbox::without_text(&mut true));
            if field(
                ui,
                &mut expr,
                expr_width,
                "res.status, res.body.id, res.headers.content-type…",
                variables,
                false,
            ) {
                assertions.push(Assertion::new(expr, AssertOp::Eq, ""));
                changed = true;
            }
            ui.end_row();
        });
    if let Some(i) = remove {
        assertions.remove(i);
        changed = true;
    }
    changed
}

fn op_picker(ui: &mut Ui, row: usize, op: &mut AssertOp) -> bool {
    let mut changed = false;
    ComboBox::from_id_salt(("assert op", row))
        .width(100.0)
        .selected_text(RichText::new(op.as_str()).monospace())
        .show_ui(ui, |ui| {
            for candidate in AssertOp::ALL {
                changed |= ui
                    .selectable_value(op, candidate, RichText::new(candidate.as_str()).monospace())
                    .changed();
            }
        });
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names() {
        assert_eq!(file_name(r"C:\a\b.png"), "b.png");
        assert_eq!(file_name("/tmp/c.json"), "c.json");
        assert_eq!(file_name("d"), "d");
    }
}
