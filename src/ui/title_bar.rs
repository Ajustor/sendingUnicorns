//! The title bar, as browsingUnicorns' tab strip: the gradient, the app's
//! name, the open requests' tabs, and the window's buttons. Its empty space
//! moves the window (double click: maximize).

use domain::{HttpMethod, RequestId};
use egui::{Button, Color32, CornerRadius, RichText, ScrollArea, Sense, TextureHandle, Ui};
use egui_phosphor::regular as icon;

use super::command::Command;
use super::theme;

/// Room left of the tabs for macOS's window buttons, drawn over the bar.
const MACOS_BUTTONS: f32 = 72.0;
/// Width of the window's buttons (Windows, Linux).
const WINDOW_BUTTONS: f32 = 3.0 * 46.0;
const TITLE_CHARS: usize = 22;

/// What a tab shows.
#[derive(Debug, Clone)]
pub struct TabInfo {
    pub id: RequestId,
    pub method: HttpMethod,
    pub name: String,
    /// Not in a collection yet.
    pub draft: bool,
    /// Its collection has unsaved changes.
    pub dirty: bool,
    pub sending: bool,
}

pub fn show(
    ctx: &egui::Context,
    logo: Option<&TextureHandle>,
    tabs: &[TabInfo],
    active: Option<RequestId>,
    commands: &mut Vec<Command>,
) {
    egui::TopBottomPanel::top("title bar")
        .exact_height(42.0)
        .frame(egui::Frame::NONE.inner_margin(egui::Margin {
            left: 8,
            right: 0,
            top: 6,
            bottom: 0,
        }))
        .show(ctx, |ui| {
            let full = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), ui.max_rect().max);
            theme::gradient(ui.painter(), full, &theme::GRADIENT);
            theme::chrome_style(ui);
            drag_area(ui, full);
            ui.spacing_mut().item_spacing.x = 3.0;
            ui.horizontal(|ui| {
                if cfg!(target_os = "macos") {
                    ui.add_space(MACOS_BUTTONS);
                }
                brand(ui, logo);
                ui.add_space(10.0);
                let controls = if cfg!(target_os = "macos") {
                    0.0
                } else {
                    WINDOW_BUTTONS
                };
                tab_strip(ui, tabs, active, controls + 36.0, commands);
                let plus = theme::hover_fill(ui, |ui| {
                    ui.add(
                        Button::new(RichText::new(icon::PLUS).color(Color32::WHITE).size(16.0))
                            .frame(false),
                    )
                })
                .on_hover_text("New request (Ctrl+T)");
                if plus.clicked() {
                    commands.push(Command::NewTab);
                }
                if !cfg!(target_os = "macos") {
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Min),
                        window_buttons,
                    );
                }
            });
        });
}

/// The icon and `sendingUnicorns`, as on the download page.
fn brand(ui: &mut Ui, logo: Option<&TextureHandle>) {
    if let Some(logo) = logo {
        ui.add(
            egui::Image::new(logo)
                .fit_to_exact_size(egui::vec2(22.0, 22.0))
                .corner_radius(5.0),
        );
    }
    ui.label(
        RichText::new("sending")
            .strong()
            .color(Color32::WHITE)
            .size(15.0),
    );
    ui.add_space(-3.0);
    ui.label(
        RichText::new("Unicorns")
            .color(theme::CHROME_TEXT_WEAK)
            .size(15.0),
    );
}

fn tab_strip(
    ui: &mut Ui,
    tabs: &[TabInfo],
    active: Option<RequestId>,
    reserved: f32,
    commands: &mut Vec<Command>,
) {
    ui.style_mut().always_scroll_the_only_direction = true;
    ScrollArea::horizontal()
        .auto_shrink([true, true])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .max_width((ui.available_width() - reserved).max(80.0))
        .drag_to_scroll(false)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let mut rects = Vec::with_capacity(tabs.len());
                let mut dragged = None;
                // A newly active tab is scrolled into view, once: the user
                // may scroll away from it.
                let shown = ui.id().with("active tab");
                let newly_active = ui.data_mut(|d| {
                    let changed = d.get_temp::<Option<RequestId>>(shown) != Some(active);
                    d.insert_temp(shown, active);
                    changed
                });
                for (index, tab) in tabs.iter().enumerate() {
                    let is_active = active == Some(tab.id);
                    let response = tab_button(ui, tab, is_active, commands);
                    if is_active && newly_active {
                        response.scroll_to_me(None);
                    }
                    if response.dragged() || response.drag_stopped() {
                        dragged = Some((index, tab.id, response.drag_stopped()));
                    }
                    rects.push(response.rect);
                }
                if let Some((from, id, dropped)) = dragged {
                    drag_tab(ui, &rects, from, id, dropped, commands);
                }
            });
        });
}

fn tab_button(
    ui: &mut Ui,
    tab: &TabInfo,
    active: bool,
    commands: &mut Vec<Command>,
) -> egui::Response {
    let (fill, text) = if active {
        (theme::NIGHT, Color32::WHITE)
    } else {
        (Color32::TRANSPARENT, theme::TAB_TEXT)
    };
    let corners = CornerRadius {
        nw: 10,
        ne: 10,
        sw: 0,
        se: 0,
    };
    let frame = egui::Frame::NONE
        .fill(fill)
        .corner_radius(corners)
        .inner_margin(egui::Margin {
            left: 10,
            right: 6,
            top: 7,
            bottom: 7,
        });
    let response = ui
        .scope_builder(
            egui::UiBuilder::new().sense(Sense::click_and_drag()),
            |ui| {
                frame.show(ui, |ui| {
                    ui.set_min_width(120.0);
                    ui.set_max_width(200.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 5.0;
                        if tab.sending {
                            ui.add(egui::Spinner::new().size(11.0).color(text));
                        }
                        ui.label(
                            RichText::new(short_method(tab.method))
                                .monospace()
                                .size(10.5)
                                .strong()
                                .color(theme::method_color(tab.method, true)),
                        );
                        let title = truncate(&tab.name, TITLE_CHARS);
                        let title = if tab.draft {
                            RichText::new(title).italics()
                        } else {
                            RichText::new(title)
                        };
                        ui.add(egui::Label::new(title.color(text)).selectable(false))
                            .on_hover_text(if tab.draft {
                                format!("{} — not saved in a collection", tab.name)
                            } else {
                                tab.name.clone()
                            });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let close = theme::hover_fill(ui, |ui| {
                                let glyph = if tab.dirty && !ui.rect_contains_pointer(ui.max_rect())
                                {
                                    RichText::new("●").color(theme::PINK).size(10.0)
                                } else {
                                    RichText::new(icon::X)
                                        .color(text.gamma_multiply(0.7))
                                        .small()
                                };
                                ui.add(Button::new(glyph).frame(false))
                            })
                            .on_hover_text("Close (Ctrl+W)");
                            if close.clicked() {
                                commands.push(Command::CloseTab(tab.id));
                            }
                        });
                    });
                })
            },
        )
        .response;
    if !active && response.contains_pointer() {
        ui.painter()
            .rect_filled(response.rect, corners, Color32::from_white_alpha(28));
    }
    if response.clicked() {
        commands.push(Command::Open(tab.id));
    }
    if response.middle_clicked() {
        commands.push(Command::CloseTab(tab.id));
    }
    response
}

/// Methods as the tabs show them: three or four letters.
fn short_method(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Delete => "DEL",
        HttpMethod::Options => "OPT",
        other => other.as_str(),
    }
}

/// A tab being dragged: a bar where it would land; dropped, it moves there.
fn drag_tab(
    ui: &Ui,
    rects: &[egui::Rect],
    from: usize,
    id: RequestId,
    dropped: bool,
    commands: &mut Vec<Command>,
) {
    let Some(pointer) = ui.ctx().pointer_latest_pos() else {
        return;
    };
    let (slot, to) = drop_slot(rects, from, pointer.x);
    if to == from {
        return;
    }
    if dropped {
        commands.push(Command::MoveTab(id, to));
        return;
    }
    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    let x = rects
        .get(slot)
        .map_or_else(|| rects[rects.len() - 1].right() + 1.5, |r| r.left() - 1.5);
    ui.painter().vline(
        x,
        rects[from].y_range(),
        egui::Stroke::new(3.0_f32, theme::CYAN),
    );
}

/// Where a tab dragged from index `from` to `x` goes: the gap it is over
/// (before tab `slot`), and its index once moved there.
fn drop_slot(rects: &[egui::Rect], from: usize, x: f32) -> (usize, usize) {
    let slot = rects.iter().filter(|r| r.center().x < x).count();
    let to = if slot > from { slot - 1 } else { slot };
    (slot, to)
}

/// The bar's empty space: dragged, it moves the window; double-clicked, it
/// maximizes or restores it.
fn drag_area(ui: &Ui, rect: egui::Rect) {
    let bar = ui.interact(rect, ui.id().with("drag"), Sense::click_and_drag());
    if bar.double_clicked() {
        let maximized = ui.ctx().input(|i| i.viewport().maximized == Some(true));
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    } else if bar.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
}

/// Close, maximize or restore, minimize (right to left).
fn window_buttons(ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let maximized = ctx.input(|i| i.viewport().maximized == Some(true));
    ui.spacing_mut().item_spacing.x = 0.0;
    let mut button = |glyph: &str, tip: &str, danger: bool| {
        let (rect, response) = ui.allocate_exact_size(egui::vec2(46.0, 36.0), Sense::click());
        let rect = egui::Rect::from_min_max(egui::pos2(rect.left(), 0.0), rect.max);
        if response.hovered() {
            let fill = if danger {
                Color32::from_rgb(0xc4, 0x2b, 0x1c)
            } else {
                Color32::from_white_alpha(36)
            };
            ui.painter().rect_filled(rect, 0.0, fill);
        }
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            glyph,
            egui::FontId::proportional(15.0),
            Color32::WHITE,
        );
        response.on_hover_text(tip).clicked()
    };
    if button(icon::X, "Close", true) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    let (glyph, tip) = if maximized {
        (icon::COPY_SIMPLE, "Restore")
    } else {
        (icon::SQUARE, "Maximize")
    };
    if button(glyph, tip, false) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    }
    if button(icon::MINUS, "Minimize", false) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
    }
}

pub fn truncate(title: &str, max: usize) -> String {
    if title.chars().count() <= max {
        return title.to_owned();
    }
    let mut short: String = title.chars().take(max - 1).collect();
    short.push('…');
    short
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dragged_tab_lands_in_the_gap_under_the_pointer() {
        let rects: Vec<egui::Rect> = (0..3_u8)
            .map(|i| {
                let left = f32::from(i) * 100.0;
                egui::Rect::from_min_max(egui::pos2(left, 0.0), egui::pos2(left + 100.0, 30.0))
            })
            .collect();
        assert_eq!(drop_slot(&rects, 0, 280.0), (3, 2));
        assert_eq!(drop_slot(&rects, 2, 20.0), (0, 0));
        assert_eq!(drop_slot(&rects, 1, 120.0).1, 1);
    }

    #[test]
    fn long_titles_are_cut() {
        assert_eq!(truncate("short", TITLE_CHARS), "short");
        let long = "a".repeat(40);
        assert_eq!(truncate(&long, TITLE_CHARS).chars().count(), TITLE_CHARS);
        assert!(truncate(&long, TITLE_CHARS).ends_with('…'));
    }
}
