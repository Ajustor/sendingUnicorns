//! The suite's look, as in browsingUnicorns: the icon's night purple and
//! cyan. The chrome (title bar with the tabs, the address bar) is always
//! dark, its title bar a violet-to-cyan gradient; panels follow the
//! system's light or dark theme.

use domain::{HttpMethod, JsonToken, StatusClass};
use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::{
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Painter, Rect, RichText, Stroke,
    Theme, ThemePreference, Ui, Visuals,
};

const fn rgb(hex: u32) -> Color32 {
    let [_, r, g, b] = hex.to_be_bytes();
    Color32::from_rgb(r, g, b)
}

pub const HAIRLINE: f32 = 1.0;

/// The title bar's gradient, left to right (browsingUnicorns' tab strip).
pub const GRADIENT: [Color32; 3] = [rgb(0x3a_246e), rgb(0x5b_3fa6), rgb(0x2a_86a8)];
/// The address bar, the active tab.
pub const NIGHT: Color32 = rgb(0x24_1a3d);
/// The address field, and its outline.
pub const NIGHT_DEEP: Color32 = rgb(0x17_1126);
pub const FIELD_STROKE: Color32 = rgb(0x3f_3270);
/// Hovered chrome buttons.
pub const NIGHT_HOVER: Color32 = rgb(0x3a_2c63);
/// Text and icons on the chrome.
pub const CHROME_TEXT: Color32 = rgb(0xe9_e4ff);
pub const CHROME_TEXT_WEAK: Color32 = rgb(0xb9_b0d8);
/// Inactive tab titles, on the gradient (white at 90 %).
pub const TAB_TEXT: Color32 = Color32::from_rgba_premultiplied(230, 230, 230, 230);
/// Focus, the Send button's end, links on the chrome.
pub const CYAN: Color32 = rgb(0x5a_dcff);
/// The unicorn's pink: unsaved marks, the seal of the icon.
pub const PINK: Color32 = rgb(0xff_79c6);

/// Panels and fields, dark theme.
mod dark {
    use super::{Color32, rgb};
    pub const PANEL: Color32 = rgb(0x1c_1430);
    pub const SIDE: Color32 = rgb(0x18_1129);
    pub const HOVER: Color32 = rgb(0x2a_1f47);
    pub const BORDER: Color32 = rgb(0x2f_2350);
    pub const FIELD: Color32 = rgb(0x14_0e24);
    pub const TEXT: Color32 = rgb(0xec_e7ff);
    pub const ACCENT: Color32 = rgb(0x5a_dcff);
}

/// Panels and fields, light theme.
mod light {
    use super::{Color32, rgb};
    pub const PANEL: Color32 = rgb(0xfa_f8ff);
    pub const SIDE: Color32 = rgb(0xf2_eefc);
    pub const HOVER: Color32 = rgb(0xef_eaff);
    pub const BORDER: Color32 = rgb(0xe6_e0f5);
    pub const FIELD: Color32 = Color32::WHITE;
    pub const FIELD_BORDER: Color32 = rgb(0xe0_d9f3);
    pub const TEXT: Color32 = rgb(0x24_1b3d);
    pub const ACCENT: Color32 = rgb(0x5b_3fa6);
}

/// Fonts and both themes' visuals; egui follows the system's theme.
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(fonts());
    ctx.set_visuals_of(Theme::Dark, visuals(Theme::Dark));
    ctx.set_visuals_of(Theme::Light, visuals(Theme::Light));
    ctx.set_theme(ThemePreference::System);
    ctx.style_mut(|s| {
        s.spacing.button_padding = egui::vec2(8.0, 4.0);
        s.spacing.item_spacing = egui::vec2(8.0, 6.0);
        s.spacing.interact_size.y = 24.0;
    });
}

fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    match super::system_font::load() {
        Some((path, bytes)) => {
            log::info!("font: {}", path.display());
            fonts.font_data.insert(
                "system".into(),
                std::sync::Arc::new(FontData::from_owned(bytes)),
            );
            // First choice; egui's own font stays as a fallback for missing glyphs.
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "system".into());
        }
        None => log::warn!("font: no system interface font, egui's own is used"),
    }
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    fonts
}

fn visuals(theme: Theme) -> Visuals {
    let (mut v, panel, hover, border, field, text, accent) = match theme {
        Theme::Dark => (
            Visuals::dark(),
            dark::PANEL,
            dark::HOVER,
            dark::BORDER,
            dark::FIELD,
            dark::TEXT,
            dark::ACCENT,
        ),
        Theme::Light => (
            Visuals::light(),
            light::PANEL,
            light::HOVER,
            light::BORDER,
            light::FIELD,
            light::TEXT,
            light::ACCENT,
        ),
    };
    let radius = CornerRadius::same(8);
    v.panel_fill = panel;
    v.window_fill = panel;
    v.extreme_bg_color = field;
    v.faint_bg_color = hover;
    v.window_stroke = Stroke::new(HAIRLINE, border);
    v.override_text_color = Some(text);
    v.hyperlink_color = accent;
    v.selection.bg_fill = accent.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(HAIRLINE, accent);
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = radius;
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = radius;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(HAIRLINE, border);
    v.widgets.inactive.weak_bg_fill = hover;
    v.widgets.inactive.bg_fill = hover;
    v.widgets.hovered.weak_bg_fill = hover;
    v.widgets.hovered.bg_fill = hover;
    v.widgets.hovered.bg_stroke = Stroke::new(HAIRLINE, accent.gamma_multiply(0.6));
    v.widgets.active.weak_bg_fill = accent.gamma_multiply(0.5);
    v.widgets.active.bg_fill = accent.gamma_multiply(0.5);
    if theme == Theme::Light {
        v.widgets.inactive.bg_stroke = Stroke::new(HAIRLINE, light::FIELD_BORDER);
    }
    v
}

/// Chrome widgets (title bar, address bar): always dark, lavender text,
/// rounded hover, cyan focus. Call at the start of their `ui`.
pub fn chrome_style(ui: &mut Ui) {
    let v = ui.visuals_mut();
    *v = visuals(Theme::Dark);
    v.override_text_color = Some(CHROME_TEXT);
    v.panel_fill = NIGHT;
    v.extreme_bg_color = NIGHT_DEEP;
    v.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    v.widgets.inactive.bg_fill = NIGHT_DEEP;
    v.widgets.inactive.bg_stroke = Stroke::new(HAIRLINE, FIELD_STROKE);
    v.widgets.hovered.weak_bg_fill = NIGHT_HOVER;
    v.widgets.hovered.bg_fill = NIGHT_DEEP;
    v.widgets.hovered.bg_stroke = Stroke::new(HAIRLINE, FIELD_STROKE);
    v.widgets.active.bg_stroke = Stroke::new(HAIRLINE, CYAN);
    v.selection.stroke = Stroke::new(HAIRLINE, CYAN);
    v.widgets.noninteractive.fg_stroke = Stroke::new(HAIRLINE, CHROME_TEXT_WEAK);
}

/// Paints `colors` over `rect`, left to right.
pub fn gradient(painter: &Painter, rect: Rect, colors: &[Color32]) {
    let mut mesh = Mesh::default();
    let stops = colors.len().saturating_sub(1).max(1);
    #[allow(clippy::cast_precision_loss)] // a handful of stops
    for (i, color) in colors.iter().enumerate() {
        let x = egui::lerp(rect.left()..=rect.right(), i as f32 / stops as f32);
        for y in [rect.top(), rect.bottom()] {
            mesh.vertices.push(Vertex {
                pos: egui::pos2(x, y),
                uv: WHITE_UV,
                color: *color,
            });
        }
    }
    #[allow(clippy::cast_possible_truncation)] // a handful of stops
    for i in 0..stops as u32 {
        let (tl, bl, tr, br) = (2 * i, 2 * i + 1, 2 * i + 2, 2 * i + 3);
        mesh.add_triangle(tl, bl, tr);
        mesh.add_triangle(tr, bl, br);
    }
    painter.add(mesh);
}

/// A button with a rounded background under the pointer (stronger while
/// pressed), and the hand cursor: frameless buttons show nothing otherwise.
pub fn hover_fill(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> egui::Response) -> egui::Response {
    // Reserved before the button, so that the background is under it.
    let background = ui.painter().add(egui::Shape::Noop);
    let response = add(ui);
    if response.hovered() && response.enabled() {
        let pressed = response.is_pointer_button_down_on();
        let fill = match (ui.visuals().dark_mode, pressed) {
            (true, false) => Color32::from_white_alpha(30),
            (true, true) => Color32::from_white_alpha(56),
            (false, false) => Color32::from_black_alpha(18),
            (false, true) => Color32::from_black_alpha(36),
        };
        ui.painter().set(
            background,
            egui::epaint::RectShape::filled(response.rect.expand(2.0), CornerRadius::same(6), fill),
        );
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// A frameless icon button with [`hover_fill`].
pub fn icon_button(ui: &mut Ui, glyph: &str, tip: &str) -> egui::Response {
    hover_fill(ui, |ui| ui.add(egui::Button::new(glyph).frame(false))).on_hover_text(tip)
}

/// A section pill; the selected one is violet, the gradient's middle.
pub fn pill(ui: &mut Ui, selected: bool, text: impl Into<String>) -> egui::Response {
    let text_color = if selected {
        Color32::WHITE
    } else {
        ui.visuals().text_color()
    };
    let button = egui::Button::new(RichText::new(text.into()).color(text_color))
        .corner_radius(CornerRadius::same(255))
        .fill(if selected {
            GRADIENT[1]
        } else {
            Color32::TRANSPARENT
        })
        .stroke(Stroke::NONE);
    let response = ui.add(button);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// A small rounded label on a tinted background (methods, statuses).
pub fn badge(ui: &mut Ui, text: &str, color: Color32, min_width: f32) -> egui::Response {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), egui::FontId::monospace(11.0), color);
    let size = egui::vec2(
        (galley.size().x + 12.0).max(min_width),
        galley.size().y + 6.0,
    );
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(255), color.gamma_multiply(0.16));
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, color);
    response
}

fn is_dark(ui: &Ui) -> bool {
    ui.visuals().dark_mode
}

/// The collections panel, a shade apart from the editor.
pub fn side_fill(dark_mode: bool) -> Color32 {
    if dark_mode { dark::SIDE } else { light::SIDE }
}

pub fn accent(ui: &Ui) -> Color32 {
    if is_dark(ui) {
        dark::ACCENT
    } else {
        light::ACCENT
    }
}

/// Each method its colour, as in other HTTP clients; `on_dark` for the
/// chrome, which is dark in both themes.
pub fn method_color(method: HttpMethod, on_dark: bool) -> Color32 {
    let (dark, light) = match method {
        HttpMethod::Get => (0x50_fa7b, 0x1a_7f37),
        HttpMethod::Post => (0xff_b86c, 0xa3_5200),
        HttpMethod::Put => (0x8b_e9fd, 0x0a_6a8c),
        HttpMethod::Patch => (0xbd_93f9, 0x6f_42c1),
        HttpMethod::Delete => (0xff_5555, 0xc0_2626),
        HttpMethod::Head | HttpMethod::Options => (0xb9_b0d8, 0x5c_5470),
    };
    rgb(if on_dark { dark } else { light })
}

pub fn method(ui: &Ui, method: HttpMethod) -> Color32 {
    method_color(method, is_dark(ui))
}

pub fn status(ui: &Ui, class: StatusClass) -> Color32 {
    let (on_dark, on_light) = match class {
        StatusClass::Informational | StatusClass::Redirection => (0x8b_e9fd, 0x0a_6a8c),
        StatusClass::Success => (0x50_fa7b, 0x1a_7f37),
        StatusClass::ClientError => (0xff_b86c, 0xa3_5200),
        StatusClass::ServerError => (0xff_5555, 0xc0_2626),
    };
    rgb(if is_dark(ui) { on_dark } else { on_light })
}

pub fn success(ui: &Ui) -> Color32 {
    rgb(if is_dark(ui) { 0x50_fa7b } else { 0x1a_7f37 })
}

pub fn error(ui: &Ui) -> Color32 {
    rgb(if is_dark(ui) { 0xff_6e6e } else { 0xc0_2626 })
}

pub fn weak(ui: &Ui) -> Color32 {
    ui.visuals().weak_text_color()
}

/// A `{{variable}}`: defined or not.
pub fn variable(ui: &Ui, defined: bool) -> Color32 {
    match (defined, is_dark(ui)) {
        (true, true) => rgb(0x50_fa7b),
        (true, false) => rgb(0x1a_7f37),
        (false, _) => error(ui),
    }
}

pub fn json(dark_mode: bool, token: JsonToken) -> Option<Color32> {
    let (on_dark, on_light) = match token {
        JsonToken::Key => (0x8b_e9fd, 0x0a_6a8c),
        JsonToken::String => (0xf1_fa8c, 0x7a_6000),
        JsonToken::Number => (0xbd_93f9, 0x6f_42c1),
        JsonToken::Literal => (0xff_79c6, 0xb3_1d7a),
        JsonToken::Punct | JsonToken::Whitespace | JsonToken::Other | JsonToken::Placeholder => {
            return None;
        }
    };
    Some(rgb(if dark_mode { on_dark } else { on_light }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(c: Color32) -> f32 {
        let channel = |v: u8| {
            let v = f32::from(v) / 255.0;
            if v <= 0.040_45 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(c.r()) + 0.7152 * channel(c.g()) + 0.0722 * channel(c.b())
    }

    fn contrast(a: Color32, b: Color32) -> f32 {
        let (la, lb) = (luminance(a), luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    /// WCAG AA for text on the field background, in both themes.
    #[test]
    fn coloured_text_is_readable() {
        for dark_mode in [true, false] {
            let field = if dark_mode { dark::FIELD } else { light::FIELD };
            for token in [
                JsonToken::Key,
                JsonToken::String,
                JsonToken::Number,
                JsonToken::Literal,
            ] {
                let color = json(dark_mode, token).unwrap();
                assert!(contrast(color, field) >= 4.5, "{token:?} dark={dark_mode}");
            }
            for method in HttpMethod::ALL {
                assert!(
                    contrast(method_color(method, dark_mode), field) >= 4.5,
                    "{method}"
                );
            }
            let text = if dark_mode { dark::TEXT } else { light::TEXT };
            assert!(contrast(text, field) >= 7.0);
        }
    }

    #[test]
    fn the_chrome_is_readable() {
        assert!(contrast(CHROME_TEXT, NIGHT_DEEP) >= 4.5);
        assert!(contrast(CHROME_TEXT, NIGHT) >= 4.5);
        assert!(contrast(CHROME_TEXT, NIGHT_HOVER) >= 4.5);
        for method in HttpMethod::ALL {
            assert!(contrast(method_color(method, true), NIGHT_DEEP) >= 4.5);
        }
        for stop in GRADIENT {
            assert!(contrast(TAB_TEXT, stop) >= 3.0);
        }
    }
}
