//! The address bar, under the title bar: always dark, as browsingUnicorns'
//! toolbar. Where the request lives, its name, the environment, then the
//! method, the address and Send.

use domain::{Collection, HttpMethod, Request, Variables};
use egui::{Button, Color32, ComboBox, CornerRadius, Key, RichText, TextEdit, TextStyle, Ui};
use egui_phosphor::regular as icon;

use super::command::Command;
use super::highlight::template_layouter;
use super::theme;

/// Where the request is shown: its collection and folders, or "Draft".
#[derive(Debug)]
pub struct Location<'a> {
    pub collection: Option<&'a Collection>,
    pub folders: Vec<String>,
    pub environment: Option<&'a str>,
}

/// Whether the request changed.
pub fn show(
    ctx: &egui::Context,
    request: &mut Request,
    location: &Location<'_>,
    variables: &Variables,
    sending: bool,
    commands: &mut Vec<Command>,
) -> bool {
    let mut changed = false;
    egui::TopBottomPanel::top("address bar")
        .frame(
            egui::Frame::NONE
                .fill(theme::NIGHT)
                .inner_margin(egui::Margin::symmetric(12, 8)),
        )
        .show(ctx, |ui| {
            theme::chrome_style(ui);
            ui.horizontal(|ui| {
                changed |= breadcrumb(ui, request, location, commands);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(collection) = location.collection {
                        if theme::icon_button(
                            ui,
                            icon::GEAR_SIX,
                            "Collection settings: auth, headers, variables",
                        )
                        .clicked()
                        {
                            commands.push(Command::CollectionSettings(collection.name.clone()));
                        }
                        environment_picker(ui, collection, location.environment, commands);
                    }
                    if theme::icon_button(ui, icon::CODE, "Copy as cURL").clicked() {
                        commands.push(Command::CopyAsCurl);
                    }
                });
            });
            ui.add_space(6.0);
            changed |= address_row(ui, request, variables, sending, commands);
        });
    changed
}

fn breadcrumb(
    ui: &mut Ui,
    request: &mut Request,
    location: &Location<'_>,
    commands: &mut Vec<Command>,
) -> bool {
    let weak = |text: &str| RichText::new(text).color(theme::CHROME_TEXT_WEAK);
    if let Some(c) = location.collection {
        ui.label(weak(&format!("{} {}", icon::FOLDERS, c.name)));
        for folder in &location.folders {
            ui.label(weak("›"));
            ui.label(weak(folder));
        }
    } else {
        let save = theme::hover_fill(ui, |ui| {
            ui.add(
                Button::new(
                    RichText::new(format!("{} Draft — save it", icon::FLOPPY_DISK))
                        .color(theme::PINK),
                )
                .frame(false),
            )
        });
        if save
            .on_hover_text("Ctrl+S: choose its collection")
            .clicked()
        {
            commands.push(Command::Save);
        }
    }
    ui.label(weak("›"));
    let width = (ui.available_width() - 320.0).max(120.0);
    ui.add(
        TextEdit::singleline(&mut request.name)
            .frame(false)
            .desired_width(width)
            .font(TextStyle::Heading)
            .text_color(Color32::WHITE),
    )
    .changed()
}

fn environment_picker(
    ui: &mut Ui,
    collection: &Collection,
    active: Option<&str>,
    commands: &mut Vec<Command>,
) {
    let text = active.map_or_else(
        || {
            RichText::new(format!("{} No environment", icon::GLOBE_SIMPLE))
                .color(theme::CHROME_TEXT_WEAK)
        },
        |name| RichText::new(format!("{} {name}", icon::GLOBE)).color(theme::CYAN),
    );
    ComboBox::from_id_salt("environment")
        .width(170.0)
        .selected_text(text)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(active.is_none(), "No environment")
                .clicked()
            {
                commands.push(Command::SetEnvironment(collection.name.clone(), None));
            }
            for env in &collection.environments {
                if ui
                    .selectable_label(active == Some(env.name.as_str()), &env.name)
                    .clicked()
                {
                    commands.push(Command::SetEnvironment(
                        collection.name.clone(),
                        Some(env.name.clone()),
                    ));
                }
            }
            ui.separator();
            if ui
                .button(format!("{} Manage environments…", icon::GEAR_SIX))
                .clicked()
            {
                commands.push(Command::ManageEnvironments(collection.name.clone()));
            }
            if ui
                .button(format!("{} Global variables…", icon::PLANET))
                .clicked()
            {
                commands.push(Command::ManageGlobals);
            }
        });
}

/// One rounded field holding the method and the address, then Send.
fn address_row(
    ui: &mut Ui,
    request: &mut Request,
    variables: &Variables,
    sending: bool,
    commands: &mut Vec<Command>,
) -> bool {
    let mut changed = false;
    let send_width = 104.0;
    let field_width = ui.available_width() - send_width - ui.spacing().item_spacing.x;
    ui.horizontal(|ui| {
        egui::Frame::NONE
            .fill(theme::NIGHT_DEEP)
            .stroke(egui::Stroke::new(theme::HAIRLINE, theme::FIELD_STROKE))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(egui::Margin::symmetric(6, 3))
            .show(ui, |ui| {
                ui.set_width(field_width - 12.0);
                ui.horizontal(|ui| {
                    changed |= method_picker(ui, &mut request.method);
                    ui.add(egui::Separator::default().vertical().spacing(4.0));
                    let mut layouter = template_layouter(variables, TextStyle::Monospace, false);
                    let url = ui.add(
                        TextEdit::singleline(&mut request.url)
                            .frame(false)
                            .hint_text("https://api.example.com/users/:id  —  {{base}}/users  —  paste a cURL command")
                            .desired_width(ui.available_width())
                            .margin(egui::Margin::symmetric(4, 5))
                            .layouter(&mut layouter),
                    );
                    if url.changed() {
                        if request.url.trim_start().starts_with("curl ") {
                            commands.push(Command::ImportCurl);
                        } else {
                            request.sync_params_from_url();
                        }
                        changed = true;
                    }
                    if url.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        commands.push(Command::Send);
                    }
                });
            });
        changed |= send_button(ui, send_width, sending, commands);
    });
    changed
}

fn method_picker(ui: &mut Ui, method: &mut HttpMethod) -> bool {
    let mut changed = false;
    ComboBox::from_id_salt("method")
        .width(84.0)
        .selected_text(
            RichText::new(method.as_str())
                .strong()
                .monospace()
                .color(theme::method_color(*method, true)),
        )
        .show_ui(ui, |ui| {
            for m in HttpMethod::ALL {
                let label = RichText::new(m.as_str())
                    .monospace()
                    .strong()
                    .color(theme::method_color(m, true));
                changed |= ui.selectable_value(method, m, label).changed();
            }
        });
    changed
}

/// Send, on the gradient's violet-to-cyan; Cancel while sending.
fn send_button(ui: &mut Ui, width: f32, sending: bool, commands: &mut Vec<Command>) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 34.0), egui::Sense::click());
    let colors = if sending {
        [theme::NIGHT_HOVER, theme::NIGHT_HOVER]
    } else if response.hovered() {
        [theme::GRADIENT[1], theme::CYAN]
    } else {
        [theme::GRADIENT[1], theme::GRADIENT[2]]
    };
    let painter = ui.painter().with_clip_rect(rect);
    // Rounded: the gradient under a rounded mask would cost a texture; a
    // pill of the left colour, then the gradient inset, reads the same.
    painter.rect_filled(rect, CornerRadius::same(10), colors[0]);
    let inner = rect.shrink2(egui::vec2(10.0, 0.0));
    theme::gradient(&painter, inner, &colors);
    painter.rect_filled(
        egui::Rect::from_min_max(egui::pos2(inner.right(), rect.top()), rect.max),
        CornerRadius {
            nw: 0,
            sw: 0,
            ne: 10,
            se: 10,
        },
        colors[1],
    );
    let label = if sending {
        format!("{} Cancel", icon::X)
    } else {
        format!("{} Send", icon::PAPER_PLANE_TILT)
    };
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(15.0),
        Color32::WHITE,
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if response.on_hover_text("Ctrl+Enter").clicked() {
        commands.push(if sending {
            Command::CancelSend
        } else {
            Command::Send
        });
    }
    false
}
