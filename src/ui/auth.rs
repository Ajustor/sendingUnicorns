//! The auth editor, for a request, a folder or a collection.

use domain::{Auth, KeyLocation, Variables};
use egui::{ComboBox, RichText, TextEdit, TextStyle, Ui};
use egui_phosphor::regular as icon;

use super::highlight::template_layouter;
use super::theme;

/// `inherited`: what [`Auth::Inherit`] resolves to, shown under it (`None`
/// where there is nothing to inherit from: a collection). Whether it changed.
pub fn editor(
    ui: &mut Ui,
    auth: &mut Auth,
    inherited: Option<&str>,
    variables: &Variables,
) -> bool {
    let mut changed = false;
    let mut kind = auth.kind_index();
    ui.horizontal(|ui| {
        ui.label("Type");
        ComboBox::from_id_salt("auth kind")
            .width(160.0)
            .selected_text(Auth::KINDS[kind])
            .show_ui(ui, |ui| {
                for (i, label) in Auth::KINDS.iter().enumerate() {
                    if i == 0 && inherited.is_none() {
                        continue;
                    }
                    ui.selectable_value(&mut kind, i, *label);
                }
            });
    });
    if kind != auth.kind_index() {
        *auth = Auth::of_kind(kind);
        changed = true;
    }
    ui.add_space(6.0);
    let row = |ui: &mut Ui, label: &str, text: &mut String, hint: &str, secret: bool| {
        ui.label(label);
        let mut layouter = template_layouter(variables, TextStyle::Body, false);
        let edit = TextEdit::singleline(text)
            .hint_text(hint)
            .desired_width(360.0)
            .password(secret);
        let edit = if secret {
            edit
        } else {
            edit.layouter(&mut layouter)
        };
        let changed = ui.add(edit).changed();
        ui.end_row();
        changed
    };
    egui::Grid::new("auth fields")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| match auth {
            Auth::Inherit => {
                ui.label(
                    RichText::new(format!(
                        "{} {}",
                        icon::ARROW_BEND_LEFT_UP,
                        inherited.unwrap_or("Nothing to inherit")
                    ))
                    .color(theme::weak(ui)),
                );
                ui.end_row();
            }
            Auth::None => {
                ui.label(RichText::new("No Authorization header is added.").color(theme::weak(ui)));
                ui.end_row();
            }
            Auth::Bearer { token } => {
                changed |= row(ui, "Token", token, "{{token}}", false);
            }
            Auth::Basic { username, password } => {
                changed |= row(ui, "Username", username, "", false);
                changed |= row(ui, "Password", password, "", true);
            }
            Auth::ApiKey {
                key,
                value,
                location,
            } => {
                changed |= row(ui, "Key", key, "X-API-Key", false);
                changed |= row(ui, "Value", value, "{{api_key}}", false);
                ui.label("Add to");
                ui.horizontal(|ui| {
                    changed |= ui
                        .radio_value(location, KeyLocation::Header, "Header")
                        .changed();
                    changed |= ui
                        .radio_value(location, KeyLocation::Query, "Query params")
                        .changed();
                });
                ui.end_row();
            }
        });
    changed
}

/// "Bearer token from folder users", for an inheriting request.
pub fn describe(auth: &Auth, from: &str) -> String {
    let kind = Auth::KINDS[auth.kind_index()];
    match auth {
        Auth::None | Auth::Inherit => format!("Inherits no auth from {from}"),
        _ => format!("Inherits {kind} from {from}"),
    }
}
