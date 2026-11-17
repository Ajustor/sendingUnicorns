//! `{{name}}` placeholders, replaced by the active environment's variables.

use std::collections::{BTreeSet, HashMap};
use std::ops::Range;

/// A piece of a template, as byte ranges of the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Text(Range<usize>),
    /// `{{ name }}`: the whole placeholder, and the trimmed name.
    Variable {
        range: Range<usize>,
        name: String,
    },
}

/// Cuts `text` into plain text and placeholders. An unclosed `{{` or an
/// empty name is plain text.
pub fn segments(text: &str) -> Vec<Segment> {
    let mut out = Vec::new();
    let mut plain_start = 0;
    let mut cursor = 0;
    while let Some(open) = text[cursor..].find("{{").map(|i| cursor + i) {
        let Some(close) = text[open + 2..].find("}}").map(|i| open + 2 + i) else {
            break;
        };
        let name = text[open + 2..close].trim();
        if name.is_empty() || name.contains('{') {
            // `{{{id}}}`: the placeholder may start one brace later.
            cursor = open + 1;
            continue;
        }
        if plain_start < open {
            out.push(Segment::Text(plain_start..open));
        }
        out.push(Segment::Variable {
            range: open..close + 2,
            name: name.to_owned(),
        });
        cursor = close + 2;
        plain_start = cursor;
    }
    if plain_start < text.len() {
        out.push(Segment::Text(plain_start..text.len()));
    }
    out
}

/// Resolved variables: values may use other variables (`{{host}}/api`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Variables(HashMap<String, String>);

/// How deep variables may refer to one another; deeper (or cyclic)
/// references stay as written.
const MAX_DEPTH: usize = 8;

impl Variables {
    /// From `(name, value)` pairs, later names winning; values are resolved
    /// against each other.
    pub fn resolve<'a>(pairs: impl IntoIterator<Item = (&'a str, &'a str)>) -> Self {
        let raw: HashMap<String, String> = pairs
            .into_iter()
            .map(|(k, v)| (k.trim().to_owned(), v.to_owned()))
            .filter(|(k, _)| !k.is_empty())
            .collect();
        let mut resolved = Self(raw);
        for _ in 0..MAX_DEPTH {
            let next: HashMap<String, String> = resolved
                .0
                .iter()
                .map(|(k, v)| (k.clone(), resolved.render(v).0))
                .collect();
            if next == resolved.0 {
                break;
            }
            resolved.0 = next;
        }
        resolved
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.0.contains_key(name)
    }

    /// `text` with its placeholders replaced, and the names it uses that
    /// are not defined (left as written).
    pub fn render(&self, text: &str) -> (String, BTreeSet<String>) {
        let mut out = String::with_capacity(text.len());
        let mut missing = BTreeSet::new();
        for segment in segments(text) {
            match segment {
                Segment::Text(range) => out.push_str(&text[range]),
                Segment::Variable { range, name } => {
                    if let Some(value) = self.0.get(&name) {
                        out.push_str(value);
                    } else {
                        out.push_str(&text[range]);
                        missing.insert(name);
                    }
                }
            }
        }
        (out, missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> Variables {
        Variables::resolve(pairs.iter().copied())
    }

    #[test]
    fn finds_placeholders() {
        let text = "a{{ x }}b{{}}c{{y";
        let s = segments(text);
        assert_eq!(
            s,
            [
                Segment::Text(0..1),
                Segment::Variable {
                    range: 1..8,
                    name: "x".into()
                },
                Segment::Text(8..text.len()),
            ]
        );
    }

    #[test]
    fn renders_and_reports_missing_names() {
        let v = vars(&[("host", "localhost"), ("port", "8080")]);
        let (text, missing) = v.render("http://{{host}}:{{port}}/{{path}}");
        assert_eq!(text, "http://localhost:8080/{{path}}");
        assert_eq!(missing.into_iter().collect::<Vec<_>>(), ["path"]);
    }

    #[test]
    fn variables_can_use_variables() {
        let v = vars(&[
            ("api", "{{base}}/v1"),
            ("base", "https://{{host}}"),
            ("host", "h"),
        ]);
        assert_eq!(v.get("api"), Some("https://h/v1"));
    }

    #[test]
    fn cycles_stop() {
        let v = vars(&[("a", "{{b}}"), ("b", "{{a}}")]);
        assert!(v.get("a").is_some_and(|a| a.contains("{{")));
    }

    #[test]
    fn a_brace_before_a_placeholder_is_text() {
        let v = vars(&[("id", "7")]);
        assert_eq!(v.render("{{{id}}}").0, "{7}");
    }

    #[test]
    fn json_braces_are_text() {
        let v = vars(&[("id", "7")]);
        assert_eq!(v.render(r#"{"a":{"id":{{id}}}}"#).0, r#"{"a":{"id":7}}"#);
    }
}
