//! The request's tabs: params, auth, headers, body, variables, assertions
//! and docs.

use domain::{BodyKind, KeyValue, Request, Variables, pretty_json};
use egui::{Button, RichText, ScrollArea, TextEdit, TextStyle, Ui};
use egui_phosphor::regular as icon;

use super::auth;
use super::highlight::{json_layouter, template_layouter};
use super::table::{self, Columns, key_values};
use super::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Tab {
    #[default]
    Params,
    Auth,
    Headers,
    Body,
    Vars,
    Assert,
    Docs,
}

/// What the request gets from its folders and collection, to show.
#[derive(Debug, Default)]
pub struct Inheritance {
    /// "Inherits Bearer token from folder users"; `None` for a draft.
    pub auth: Option<String>,
    pub headers: Vec<KeyValue>,
}

#[derive(Debug, Default)]
pub struct Editor {
    tab: Tab,
}

impl Editor {
    /// Whether the request changed.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        request: &mut Request,
        variables: &Variables,
        inheritance: &Inheritance,
    ) -> bool {
        let mut changed = false;
        self.tabs(ui, request);
        ui.add_space(2.0);
        ui.separator();
        ScrollArea::vertical()
            .id_salt("config")
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.add_space(4.0);
                match self.tab {
                    Tab::Params => changed |= params(ui, request, variables),
                    Tab::Auth => {
                        let inherited = inheritance
                            .auth
                            .as_deref()
                            .or(Some("A draft inherits nothing."));
                        changed |= auth::editor(ui, &mut request.auth, inherited, variables);
                    }
                    Tab::Headers => {
                        changed |= headers(ui, request, variables, &inheritance.headers);
                    }
                    Tab::Body => changed |= body(ui, request, variables),
                    Tab::Vars => changed |= vars(ui, request, variables),
                    Tab::Assert => {
                        hint(
                            ui,
                            "Checked on every response; see the Tests tab of the response.",
                        );
                        changed |= table::assertions(ui, &mut request.assertions, variables);
                    }
                    Tab::Docs => {
                        hint(ui, "Notes for whoever uses this request (saved with it).");
                        changed |= ui
                            .add(
                                TextEdit::multiline(&mut request.docs)
                                    .hint_text("What it does, what it needs, what it returns…")
                                    .desired_rows(14)
                                    .desired_width(f32::INFINITY),
                            )
                            .changed();
                    }
                }
            });
        changed
    }

    fn tabs(&mut self, ui: &mut Ui, request: &Request) {
        let count = |n: usize| {
            if n == 0 {
                String::new()
            } else {
                format!(" {n}")
            }
        };
        let body = match request.body.kind {
            BodyKind::None => String::new(),
            kind => format!(" · {}", kind.label()),
        };
        let auth = if request.auth == domain::Auth::Inherit {
            ""
        } else {
            " ●"
        };
        let docs = if request.docs.trim().is_empty() {
            "Docs"
        } else {
            "Docs ●"
        };
        let tabs = [
            (
                Tab::Params,
                format!(
                    "Params{}",
                    count(request.params.active().count() + request.path_params.0.len())
                ),
            ),
            (Tab::Auth, format!("Auth{auth}")),
            (
                Tab::Headers,
                format!("Headers{}", count(request.headers.active().count())),
            ),
            (Tab::Body, format!("Body{body}")),
            (
                Tab::Vars,
                format!(
                    "Vars{}",
                    count(request.vars.active().count() + request.post_vars.active().count())
                ),
            ),
            (
                Tab::Assert,
                format!(
                    "Assert{}",
                    count(request.assertions.iter().filter(|a| a.enabled).count())
                ),
            ),
            (Tab::Docs, docs.to_owned()),
        ];
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for (tab, label) in tabs {
                if theme::pill(ui, self.tab == tab, label).clicked() {
                    self.tab = tab;
                }
            }
        });
    }
}

pub fn hint(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).small().color(theme::weak(ui)));
    ui.add_space(4.0);
}

pub fn heading(ui: &mut Ui, text: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(text).small().strong().color(theme::weak(ui)));
}

fn params(ui: &mut Ui, request: &mut Request, variables: &Variables) -> bool {
    let mut changed = false;
    heading(ui, "QUERY PARAMS");
    if key_values(
        ui,
        "params",
        &mut request.params,
        variables,
        Columns::new("Parameter", "Value"),
    ) {
        request.sync_url_from_params();
        changed = true;
    }
    heading(ui, "PATH VARIABLES");
    if request.path_params.is_empty() {
        hint(
            ui,
            "Write :name in the address (/users/:id) to get a value to fill here.",
        );
    } else {
        let columns = Columns {
            fixed: true,
            ..Columns::new("", "Value")
        };
        changed |= key_values(
            ui,
            "path params",
            &mut request.path_params,
            variables,
            columns,
        );
    }
    changed
}

fn headers(
    ui: &mut Ui,
    request: &mut Request,
    variables: &Variables,
    inherited: &[KeyValue],
) -> bool {
    let changed = key_values(
        ui,
        "headers",
        &mut request.headers,
        variables,
        Columns::new("Header", "Value"),
    );
    if !inherited.is_empty() {
        heading(ui, "INHERITED FROM THE COLLECTION AND FOLDERS");
        egui::Grid::new("inherited headers")
            .num_columns(2)
            .spacing([16.0, 4.0])
            .show(ui, |ui| {
                for kv in inherited {
                    ui.label(RichText::new(&kv.key).color(theme::weak(ui)));
                    ui.label(RichText::new(&kv.value).color(theme::weak(ui)));
                    ui.end_row();
                }
            });
    }
    changed
}

fn vars(ui: &mut Ui, request: &mut Request, variables: &Variables) -> bool {
    let mut changed = false;
    heading(ui, "PRE-REQUEST");
    hint(
        ui,
        "Variables for this request only; they win over the environment's.",
    );
    changed |= key_values(
        ui,
        "pre vars",
        &mut request.vars,
        variables,
        Columns::new("Variable", "Value"),
    );
    heading(ui, "POST-RESPONSE");
    hint(
        ui,
        "Set from the response, for the next requests: res.body.token, res.headers.location, res.status…",
    );
    changed |= key_values(
        ui,
        "post vars",
        &mut request.post_vars,
        variables,
        Columns::new("Variable", "res.body.token"),
    );
    changed
}

fn code(ui: &mut Ui, text: &mut String, hint: &str, json: bool, variables: &Variables) -> bool {
    let mut json_layout = json_layouter(variables);
    let mut template_layout = template_layouter(variables, TextStyle::Monospace, true);
    let edit = TextEdit::multiline(text)
        .code_editor()
        .hint_text(hint)
        .desired_rows(16)
        .desired_width(f32::INFINITY);
    let edit = if json {
        edit.layouter(&mut json_layout)
    } else {
        edit.layouter(&mut template_layout)
    };
    ui.add(edit).changed()
}

fn body(ui: &mut Ui, request: &mut Request, variables: &Variables) -> bool {
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for kind in BodyKind::ALL {
            if theme::pill(ui, request.body.kind == kind, kind.label()).clicked()
                && request.body.kind != kind
            {
                request.body.kind = kind;
                changed = true;
            }
        }
    });
    ui.add_space(6.0);
    let b = &mut request.body;
    match b.kind {
        BodyKind::None => hint(ui, "This request sends no body."),
        BodyKind::Json => {
            changed |= format_button(ui, &mut b.json);
            changed |= code(
                ui,
                &mut b.json,
                "{\n  \"name\": \"{{user}}\"\n}",
                true,
                variables,
            );
        }
        BodyKind::Form => {
            changed |= key_values(
                ui,
                "form",
                &mut b.form,
                variables,
                Columns::new("Field", "Value"),
            );
        }
        BodyKind::Multipart => changed |= table::parts(ui, &mut b.multipart, variables),
        BodyKind::Text => {
            hint(
                ui,
                "Sent as text/plain unless a Content-Type header says otherwise.",
            );
            changed |= code(ui, &mut b.text, "Any text", false, variables);
        }
        BodyKind::Xml => {
            changed |= code(
                ui,
                &mut b.xml,
                "<user><name>{{user}}</name></user>",
                false,
                variables,
            );
        }
        BodyKind::GraphQl => {
            heading(ui, "QUERY");
            changed |= code(
                ui,
                &mut b.graphql_query,
                "query {\n  user(id: \"{{id}}\") {\n    name\n  }\n}",
                false,
                variables,
            );
            heading(ui, "VARIABLES (JSON)");
            changed |= format_button(ui, &mut b.graphql_variables);
            changed |= code(
                ui,
                &mut b.graphql_variables,
                "{ \"id\": 7 }",
                true,
                variables,
            );
        }
        BodyKind::File => {
            let label = if b.file.is_empty() {
                format!("{} Choose the file to send…", icon::FILE_ARROW_UP)
            } else {
                format!("{} {}", icon::FILE, table::file_name(&b.file))
            };
            if ui.add(Button::new(label)).on_hover_text(&b.file).clicked()
                && let Some(path) = rfd::FileDialog::new().pick_file()
            {
                b.file = path.to_string_lossy().into_owned();
                changed = true;
            }
            hint(
                ui,
                "Its bytes are sent as they are; the Content-Type follows its extension.",
            );
        }
    }
    changed
}

/// Indents JSON in place, when it is JSON enough.
fn format_button(ui: &mut Ui, text: &mut String) -> bool {
    let pretty = pretty_json(text);
    let mut changed = false;
    ui.horizontal(|ui| {
        let button = ui.add_enabled(
            pretty.is_some(),
            Button::new(format!("{} Format", icon::BRACKETS_CURLY)),
        );
        if let Some(pretty) = pretty
            && button.clicked()
            && pretty != *text
        {
            *text = pretty;
            changed = true;
        }
    });
    changed
}
