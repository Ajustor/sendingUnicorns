//! The response panel: status, time, size, then body (with search),
//! headers, cookies and the assertions' results.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use domain::{AfterResponse, ContentKind, Response, pretty_json};
use egui::{RichText, ScrollArea, TextEdit, Ui};
use egui_phosphor::regular as icon;

use super::command::Command;
use super::highlight::{matches, response_layouter};
use super::preview::{Preview, previewable};
use super::theme;

/// Bodies longer than this are cut on screen (saving keeps them whole).
const DISPLAY_LIMIT: usize = 2 * 1024 * 1024;

/// What came back for one send, with its body ready to show.
#[derive(Debug)]
pub struct Exchange {
    /// Tells this exchange from the next one of the same request.
    pub key: u64,
    /// Where it was sent (relative links of an HTML preview).
    pub url: String,
    pub result: Result<Response, String>,
    pub missing_variables: BTreeSet<String>,
    pub after: AfterResponse,
    raw: String,
    /// `Some` for JSON that could be indented.
    pretty: Option<String>,
    truncated: bool,
}

impl Exchange {
    pub fn new(
        url: String,
        result: Result<Response, String>,
        missing_variables: BTreeSet<String>,
        after: AfterResponse,
    ) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let (raw, pretty, truncated) = match &result {
            Ok(response) => {
                let kind = response.content_kind();
                let mut raw = match kind {
                    ContentKind::Binary => format!(
                        "({} bytes of binary data: save it to a file to open it)",
                        response.body.len()
                    ),
                    _ => response.text(),
                };
                let truncated = raw.len() > DISPLAY_LIMIT;
                if truncated {
                    let mut end = DISPLAY_LIMIT;
                    while !raw.is_char_boundary(end) {
                        end -= 1;
                    }
                    raw.truncate(end);
                }
                let pretty = (kind == ContentKind::Json && !truncated)
                    .then(|| pretty_json(&raw))
                    .flatten();
                (raw, pretty, truncated)
            }
            Err(_) => (String::new(), None, false),
        };
        Self {
            key: NEXT.fetch_add(1, Ordering::Relaxed),
            url,
            result,
            missing_variables,
            after,
            raw,
            pretty,
            truncated,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Tab {
    #[default]
    Body,
    Preview,
    Headers,
    Cookies,
    Tests,
}

#[derive(Debug)]
pub struct ResponseView {
    tab: Tab,
    pretty: bool,
    find: String,
}

impl Default for ResponseView {
    fn default() -> Self {
        Self {
            tab: Tab::Body,
            pretty: true,
            find: String::new(),
        }
    }
}

impl ResponseView {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        exchange: Option<&Exchange>,
        sending: Option<Duration>,
        preview: &mut Preview,
        commands: &mut Vec<Command>,
    ) {
        if let Some(elapsed) = sending {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.add(egui::Spinner::new().size(28.0).color(theme::accent(ui)));
                ui.label(format!("Sending… {:.1} s", elapsed.as_secs_f32()));
            });
            ui.ctx().request_repaint_after(Duration::from_millis(100));
            return;
        }
        let Some(exchange) = exchange else {
            empty(ui);
            return;
        };
        if !exchange.missing_variables.is_empty() {
            let names: Vec<_> = exchange
                .missing_variables
                .iter()
                .map(|n| format!("{{{{{n}}}}}"))
                .collect();
            ui.label(
                RichText::new(format!(
                    "{} Not defined, sent as written: {}",
                    icon::WARNING,
                    names.join(", ")
                ))
                .color(theme::error(ui)),
            );
        }
        let response = match &exchange.result {
            Ok(response) => response,
            Err(error) => {
                ui.add_space(12.0);
                egui::Frame::NONE
                    .fill(theme::error(ui).gamma_multiply(0.12))
                    .corner_radius(egui::CornerRadius::same(10))
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(format!("{} {error}", icon::WARNING_CIRCLE))
                                .color(theme::error(ui)),
                        );
                    });
                return;
            }
        };
        let can_preview = previewable(response).is_some();
        // Binary bodies (images, PDF) have nothing to read: their preview.
        self.tab = match self.tab {
            Tab::Body if can_preview && response.content_kind() == ContentKind::Binary => {
                Tab::Preview
            }
            Tab::Preview if !can_preview => Tab::Body,
            tab => tab,
        };
        self.summary(ui, response, &exchange.after, commands);
        ui.add_space(2.0);
        ui.separator();
        match self.tab {
            Tab::Body => self.body(ui, exchange),
            Tab::Preview => {
                let rect = ui.available_rect_before_wrap();
                ui.allocate_rect(rect, egui::Sense::hover());
                preview.show(ui.ctx(), rect, exchange.key, &exchange.url, response);
                if let Some(error) = &preview.error {
                    ui.label(RichText::new(error).color(theme::error(ui)));
                }
            }
            Tab::Headers => headers(ui, response),
            Tab::Cookies => cookies(ui, response),
            Tab::Tests => tests(ui, &exchange.after),
        }
    }

    fn summary(
        &mut self,
        ui: &mut Ui,
        response: &Response,
        after: &AfterResponse,
        commands: &mut Vec<Command>,
    ) {
        ui.horizontal(|ui| {
            let color = theme::status(ui, response.status_class());
            theme::badge(
                ui,
                &format!("{} {}", response.status, response.reason),
                color,
                0.0,
            );
            ui.label(
                RichText::new(format!(
                    "{} {}",
                    icon::TIMER,
                    format_duration(response.elapsed)
                ))
                .color(theme::weak(ui)),
            );
            ui.label(
                RichText::new(format!(
                    "{} {}",
                    icon::DATABASE,
                    format_size(response.body.len())
                ))
                .color(theme::weak(ui)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::icon_button(ui, icon::FLOPPY_DISK, "Save the body to a file").clicked() {
                    commands.push(Command::SaveResponseBody);
                }
            });
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let cookies = response.cookies().len();
            let tests = if after.results.is_empty() {
                "Tests".to_owned()
            } else {
                format!("Tests {}/{}", after.passed(), after.results.len())
            };
            let preview = previewable(response).is_some();
            let tabs = [
                (Tab::Body, "Body".to_owned()),
                (
                    Tab::Preview,
                    format!("{} Preview", egui_phosphor::regular::EYE),
                ),
                (Tab::Headers, format!("Headers {}", response.headers.len())),
                (
                    Tab::Cookies,
                    if cookies == 0 {
                        "Cookies".to_owned()
                    } else {
                        format!("Cookies {cookies}")
                    },
                ),
                (Tab::Tests, tests),
            ];
            for (tab, label) in tabs {
                if tab == Tab::Preview && !preview {
                    continue;
                }
                let label = if tab == Tab::Tests && after.failed() > 0 {
                    format!("{label} {}", icon::X_CIRCLE)
                } else {
                    label
                };
                if theme::pill(ui, self.tab == tab, label).clicked() {
                    self.tab = tab;
                }
            }
        });
    }

    fn body(&mut self, ui: &mut Ui, exchange: &Exchange) {
        let shown = match (&exchange.pretty, self.pretty) {
            (Some(pretty), true) => pretty.as_str(),
            _ => exchange.raw.as_str(),
        };
        ui.horizontal(|ui| {
            if exchange.pretty.is_some() {
                ui.spacing_mut().item_spacing.x = 2.0;
                if theme::pill(ui, self.pretty, "Pretty").clicked() {
                    self.pretty = true;
                }
                if theme::pill(ui, !self.pretty, "Raw").clicked() {
                    self.pretty = false;
                }
                ui.add_space(8.0);
            }
            ui.add(
                TextEdit::singleline(&mut self.find)
                    .hint_text(format!("{} Find", icon::MAGNIFYING_GLASS))
                    .desired_width(160.0),
            );
            if !self.find.is_empty() {
                let count = matches(shown, &self.find).len();
                ui.label(
                    RichText::new(format!("{count} found"))
                        .small()
                        .color(theme::weak(ui)),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::icon_button(ui, icon::COPY, "Copy the body").clicked() {
                    ui.ctx().copy_text(shown.to_owned());
                }
            });
        });
        if exchange.truncated {
            ui.label(
                RichText::new("Cut at 2 MB on screen: save it for the whole body.")
                    .small()
                    .color(theme::weak(ui)),
            );
        }
        let json = exchange
            .result
            .as_ref()
            .is_ok_and(|r| r.content_kind() == ContentKind::Json);
        let mut layouter = response_layouter(json, &self.find);
        ScrollArea::both()
            .id_salt("response body")
            .auto_shrink(false)
            .show(ui, |ui| {
                let mut text = shown;
                ui.add(
                    TextEdit::multiline(&mut text)
                        .code_editor()
                        .frame(false)
                        .desired_width(f32::INFINITY)
                        .layouter(&mut layouter),
                );
            });
    }
}

fn empty(ui: &mut Ui) {
    ui.add_space(ui.available_height() / 4.0);
    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new(icon::PAPER_PLANE_TILT)
                .size(44.0)
                .color(theme::accent(ui).gamma_multiply(0.7)),
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new("Send the request to see its response here.").color(theme::weak(ui)),
        );
        ui.add_space(4.0);
        for (keys, what) in [
            ("Ctrl+Enter", "send"),
            ("Ctrl+S", "save"),
            ("Ctrl+T", "new request"),
            ("Ctrl+W", "close the tab"),
        ] {
            ui.label(
                RichText::new(format!("{keys}  {what}"))
                    .small()
                    .monospace()
                    .color(theme::weak(ui)),
            );
        }
    });
}

fn table(ui: &mut Ui, id: &str, rows: impl Iterator<Item = (String, String)>) {
    ScrollArea::vertical()
        .id_salt(id)
        .auto_shrink(false)
        .show(ui, |ui| {
            egui::Grid::new(id)
                .striped(true)
                .num_columns(2)
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    for (name, value) in rows {
                        ui.label(RichText::new(name).strong());
                        ui.add(egui::Label::new(value).wrap());
                        ui.end_row();
                    }
                });
        });
}

fn headers(ui: &mut Ui, response: &Response) {
    table(ui, "response headers", response.headers.iter().cloned());
}

fn cookies(ui: &mut Ui, response: &Response) {
    let cookies = response.cookies();
    if cookies.is_empty() {
        ui.label(RichText::new("This response set no cookie.").color(theme::weak(ui)));
        return;
    }
    table(
        ui,
        "response cookies",
        cookies.into_iter().map(|c| {
            let mut details = vec![c.value];
            details.extend(c.domain.map(|d| format!("domain {d}")));
            details.extend(c.path.map(|p| format!("path {p}")));
            details.extend(c.expires.map(|e| format!("expires {e}")));
            if c.http_only {
                details.push("HttpOnly".into());
            }
            if c.secure {
                details.push("Secure".into());
            }
            (c.name, details.join(" · "))
        }),
    );
}

fn tests(ui: &mut Ui, after: &AfterResponse) {
    if after.results.is_empty() && after.variables.is_empty() {
        ui.label(
            RichText::new("No assertion: add some in the request's Assert tab.")
                .color(theme::weak(ui)),
        );
        return;
    }
    ScrollArea::vertical()
        .id_salt("tests")
        .auto_shrink(false)
        .show(ui, |ui| {
            for result in &after.results {
                ui.horizontal(|ui| {
                    if result.passed {
                        ui.label(RichText::new(icon::CHECK_CIRCLE).color(theme::success(ui)));
                    } else {
                        ui.label(RichText::new(icon::X_CIRCLE).color(theme::error(ui)));
                    }
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
            if !after.variables.is_empty() {
                ui.add_space(8.0);
                ui.label(
                    RichText::new("VARIABLES SET")
                        .small()
                        .strong()
                        .color(theme::weak(ui)),
                );
                for (name, value) in &after.variables {
                    ui.label(RichText::new(format!("{name} = {value}")).monospace());
                }
            }
        });
}

fn format_duration(d: Duration) -> String {
    let ms = d.as_millis();
    if ms < 1000 {
        format!("{ms} ms")
    } else {
        format!("{:.2} s", d.as_secs_f64())
    }
}

#[allow(clippy::cast_precision_loss)] // sizes shown to one decimal
fn format_size(bytes: usize) -> String {
    match bytes {
        0..1024 => format!("{bytes} B"),
        1024..1_048_576 => format!("{:.1} KB", bytes as f64 / 1024.0),
        _ => format!("{:.1} MB", bytes as f64 / 1_048_576.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(content_type: &str, body: &[u8]) -> Response {
        Response {
            status: 200,
            reason: "OK".into(),
            headers: vec![("Content-Type".into(), content_type.into())],
            body: body.to_vec(),
            elapsed: Duration::from_millis(12),
        }
    }

    fn exchange(r: Response) -> Exchange {
        Exchange::new(
            String::new(),
            Ok(r),
            BTreeSet::new(),
            AfterResponse::default(),
        )
    }

    #[test]
    fn json_is_indented_once() {
        let e = exchange(response("application/json", br#"{"a":1}"#));
        assert_eq!(e.pretty.as_deref(), Some("{\n  \"a\": 1\n}"));
        let e = exchange(response("text/plain", b"a: 1"));
        assert!(e.pretty.is_none());
    }

    #[test]
    fn huge_bodies_are_cut_on_a_char_boundary() {
        let body = "é".repeat(DISPLAY_LIMIT);
        let e = exchange(response("text/plain", body.as_bytes()));
        assert!(e.truncated && e.raw.len() <= DISPLAY_LIMIT);
    }

    #[test]
    fn formats() {
        assert_eq!(format_duration(Duration::from_millis(250)), "250 ms");
        assert_eq!(format_duration(Duration::from_millis(1500)), "1.50 s");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(2048), "2.0 KB");
        assert_eq!(format_size(3 * 1_048_576), "3.0 MB");
    }
}
