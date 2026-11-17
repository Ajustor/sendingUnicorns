//! A request as the user writes it: templates still unresolved.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::assertions::Assertion;
use crate::auth::Auth;
use crate::key_value::{KeyValue, KeyValues};
use crate::query::{join_query, split_query};

/// Identifies a request or a folder while the app runs (not saved: files
/// hold names and order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RequestId(u64);

impl RequestId {
    /// A fresh id, never handed out before in this run.
    pub fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl HttpMethod {
    pub const ALL: [Self; 7] = [
        Self::Get,
        Self::Post,
        Self::Put,
        Self::Patch,
        Self::Delete,
        Self::Head,
        Self::Options,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        }
    }

    /// `get`, `GET`, `Get`… `None` for anything else.
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|m| m.as_str().eq_ignore_ascii_case(text.trim()))
    }
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Which of the body's contents is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BodyKind {
    #[default]
    None,
    Json,
    /// `application/x-www-form-urlencoded`.
    Form,
    /// `multipart/form-data`: text fields and files.
    Multipart,
    /// Any text, `text/plain` unless the headers say otherwise.
    Text,
    Xml,
    /// A query and its variables, sent as JSON.
    GraphQl,
    /// A file's bytes, as they are.
    File,
}

impl BodyKind {
    pub const ALL: [Self; 8] = [
        Self::None,
        Self::Json,
        Self::Form,
        Self::Multipart,
        Self::Text,
        Self::Xml,
        Self::GraphQl,
        Self::File,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Json => "JSON",
            Self::Form => "Form URL-encoded",
            Self::Multipart => "Multipart",
            Self::Text => "Text",
            Self::Xml => "XML",
            Self::GraphQl => "GraphQL",
            Self::File => "File",
        }
    }
}

/// A field of a multipart body: text, or a file's path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Part {
    pub key: String,
    /// The text, or the file's path when `is_file`.
    pub value: String,
    pub is_file: bool,
    pub enabled: bool,
}

/// Every kind's content is kept: switching kind loses nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Body {
    pub kind: BodyKind,
    pub json: String,
    pub form: KeyValues,
    pub multipart: Vec<Part>,
    pub text: String,
    pub xml: String,
    pub graphql_query: String,
    /// JSON, may be empty.
    pub graphql_variables: String,
    /// The path of the file sent as the body.
    pub file: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub id: RequestId,
    pub name: String,
    pub method: HttpMethod,
    /// The address as typed, query included (enabled params only).
    pub url: String,
    /// The query as a table; disabled rows live only here.
    pub params: KeyValues,
    /// The values of the address's `:name` segments.
    pub path_params: KeyValues,
    pub headers: KeyValues,
    pub body: Body,
    pub auth: Auth,
    /// Variables set before sending, for this request only.
    pub vars: KeyValues,
    /// After the response: variable name = expression (`res.body.token`),
    /// kept for the next requests.
    pub post_vars: KeyValues,
    pub assertions: Vec<Assertion>,
    pub docs: String,
}

impl Request {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: RequestId::next(),
            name: name.into(),
            method: HttpMethod::Get,
            url: String::new(),
            params: KeyValues::default(),
            path_params: KeyValues::default(),
            headers: KeyValues::default(),
            body: Body::default(),
            auth: Auth::Inherit,
            vars: KeyValues::default(),
            post_vars: KeyValues::default(),
            assertions: Vec::new(),
            docs: String::new(),
        }
    }

    /// A copy with a new id (same content).
    #[must_use]
    pub fn duplicate(&self, name: impl Into<String>) -> Self {
        Self {
            id: RequestId::next(),
            name: name.into(),
            ..self.clone()
        }
    }

    /// The address was edited: its query becomes the enabled params (the
    /// disabled ones stay after them), its `:segments` the path params.
    pub fn sync_params_from_url(&mut self) {
        let (_, parsed) = split_query(&self.url);
        let disabled = self.params.0.iter().filter(|kv| !kv.enabled).cloned();
        self.params = parsed.into_iter().chain(disabled).collect();
        self.sync_path_params();
    }

    /// The params table was edited: the address's query follows it.
    pub fn sync_url_from_params(&mut self) {
        let (base, _) = split_query(&self.url);
        self.url = join_query(&base, &self.params);
    }

    /// Adds an enabled param and puts it in the address.
    pub fn add_param(&mut self, key: &str, value: &str) {
        self.params.0.push(KeyValue::new(key, value));
        self.sync_url_from_params();
    }

    /// One row per `:name` of the address, in order; values kept by name.
    pub fn sync_path_params(&mut self) {
        let old = std::mem::take(&mut self.path_params);
        self.path_params = path_variable_names(&self.url)
            .into_iter()
            .map(|name| {
                let value = old
                    .0
                    .iter()
                    .find(|kv| kv.key == name)
                    .map(|kv| kv.value.clone());
                KeyValue::new(name, value.unwrap_or_default())
            })
            .collect();
    }
}

/// The names of the `:name` segments of an address's path
/// (`/users/:id/posts/:post` → `id`, `post`).
pub fn path_variable_names(url: &str) -> Vec<String> {
    let after_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
    let path = after_scheme.find('/').map_or("", |i| &after_scheme[i..]);
    let path = path.split(['?', '#']).next().unwrap_or_default();
    let mut names: Vec<String> = Vec::new();
    for segment in path.split('/') {
        if let Some(name) = segment.strip_prefix(':')
            && !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            && !names.iter().any(|n| n == name)
        {
            names.push(name.to_owned());
        }
    }
    names
}

/// `url` with each `:name` segment of its path replaced by its value (the
/// segments without a value stay).
pub fn fill_path_variables(url: &str, values: &[(String, String)]) -> String {
    let (prefix, rest) = match url.find("://") {
        Some(i) => {
            let host_end = url[i + 3..].find('/').map_or(url.len(), |j| i + 3 + j);
            url.split_at(host_end)
        }
        None => ("", url),
    };
    let tail_start = rest.find(['?', '#']).unwrap_or(rest.len());
    let (path, tail) = rest.split_at(tail_start);
    let path: Vec<String> = path
        .split('/')
        .map(|segment| {
            segment
                .strip_prefix(':')
                .and_then(|name| values.iter().find(|(k, v)| k == name && !v.is_empty()))
                .map_or_else(|| segment.to_owned(), |(_, v)| v.clone())
        })
        .collect();
    format!("{prefix}{}{tail}", path.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_parsing_ignores_case() {
        assert_eq!(HttpMethod::parse("patch"), Some(HttpMethod::Patch));
        assert_eq!(HttpMethod::parse(" Delete "), Some(HttpMethod::Delete));
        assert_eq!(HttpMethod::parse("BREW"), None);
    }

    #[test]
    fn ids_are_unique() {
        assert_ne!(Request::new("a").id, Request::new("a").id);
    }

    #[test]
    fn editing_the_url_keeps_disabled_params() {
        let mut r = Request::new("r");
        r.params = KeyValues(vec![KeyValue {
            enabled: false,
            ..KeyValue::new("debug", "1")
        }]);
        r.url = "https://h/x?page=2".into();
        r.sync_params_from_url();
        assert_eq!(
            r.params.0,
            [
                KeyValue::new("page", "2"),
                KeyValue {
                    enabled: false,
                    ..KeyValue::new("debug", "1")
                }
            ]
        );
    }

    #[test]
    fn editing_params_rewrites_the_query() {
        let mut r = Request::new("r");
        r.url = "https://h/x?old=1".into();
        r.params = KeyValues(vec![KeyValue::new("new", "2")]);
        r.sync_url_from_params();
        assert_eq!(r.url, "https://h/x?new=2");
        r.add_param("q", "{{term}}");
        assert_eq!(r.url, "https://h/x?new=2&q={{term}}");
    }

    #[test]
    fn path_variables_follow_the_address() {
        assert_eq!(
            path_variable_names("{{base}}/users/:id/posts/:post?x=:no"),
            ["id", "post"]
        );
        assert_eq!(
            path_variable_names("http://localhost:3000/a"),
            Vec::<String>::new()
        );
        let mut r = Request::new("r");
        r.url = "http://h/users/:id".into();
        r.sync_params_from_url();
        r.path_params.0[0].value = "7".into();
        r.url = "http://h/users/:id/posts/:post".into();
        r.sync_params_from_url();
        assert_eq!(
            r.path_params.0,
            [KeyValue::new("id", "7"), KeyValue::new("post", "")]
        );
    }

    #[test]
    fn path_variables_are_filled() {
        let values = [
            ("id".to_owned(), "7".to_owned()),
            ("p".to_owned(), String::new()),
        ];
        assert_eq!(
            fill_path_variables("http://localhost:3000/users/:id/:p?q=:id", &values),
            "http://localhost:3000/users/7/:p?q=:id"
        );
        assert_eq!(fill_path_variables("/a/:id", &values), "/a/7");
    }

    #[test]
    fn a_duplicate_is_another_request() {
        let r = Request::new("a");
        let copy = r.duplicate("a copy");
        assert_ne!(r.id, copy.id);
        assert_eq!(copy.name, "a copy");
    }
}
