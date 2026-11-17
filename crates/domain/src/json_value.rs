//! A small JSON reader: enough to look into response bodies
//! (`res.body.items[0].id`) without a dependency.

use std::fmt::{self, Write as _};

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    /// In file order.
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    /// `None` when `text` is not one JSON value (trailing text included).
    pub fn parse(text: &str) -> Option<Self> {
        let mut parser = Parser {
            bytes: text.as_bytes(),
            text,
            at: 0,
        };
        let value = parser.value(0)?;
        parser.skip_whitespace();
        (parser.at == parser.bytes.len()).then_some(value)
    }

    /// The value at `path`: `a.b[0].c`, `[2]`, `items.0.name` (an empty path
    /// is the value itself).
    pub fn at(&self, path: &str) -> Option<&Self> {
        let mut current = self;
        for step in path_steps(path) {
            current = match (current, step) {
                (Self::Object(fields), Step::Key(key)) => {
                    &fields.iter().rev().find(|(k, _)| k == key)?.1
                }
                (Self::Array(items), Step::Index(i)) => items.get(i)?,
                (Self::Array(items), Step::Key(key)) => items.get(key.parse::<usize>().ok()?)?,
                _ => return None,
            };
        }
        Some(current)
    }

    /// As shown and compared: strings without quotes, the rest as JSON.
    pub fn to_text(&self) -> String {
        match self {
            Self::String(s) => s.clone(),
            other => other.to_string(),
        }
    }
}

impl fmt::Display for JsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("null"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Number(n) => write!(f, "{}", format_number(*n)),
            Self::String(s) => f.write_str(&quote(s)),
            Self::Array(items) => {
                f.write_str("[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "{item}")?;
                }
                f.write_str("]")
            }
            Self::Object(fields) => {
                f.write_str("{")?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "{}:{v}", quote(k))?;
                }
                f.write_str("}")
            }
        }
    }
}

/// `42` rather than `42.0`.
#[allow(clippy::cast_possible_truncation, clippy::float_cmp)] // checked: integral and in range
pub fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

/// `text` as a JSON string, quotes included.
pub fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32); // writing to a String can't fail
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[derive(Debug, PartialEq)]
enum Step<'a> {
    Key(&'a str),
    Index(usize),
}

fn path_steps(path: &str) -> Vec<Step<'_>> {
    let mut steps = Vec::new();
    for part in path.split('.').filter(|p| !p.is_empty()) {
        let (key, mut rest) = match part.find('[') {
            Some(i) => (&part[..i], &part[i..]),
            None => (part, ""),
        };
        if !key.is_empty() {
            steps.push(Step::Key(key));
        }
        while let Some(end) = rest.find(']') {
            let inner = rest[1..end].trim_matches(|c| c == '"' || c == '\'');
            steps.push(match inner.parse() {
                Ok(i) => Step::Index(i),
                Err(_) => Step::Key(inner),
            });
            rest = &rest[end + 1..];
        }
    }
    steps
}

/// Nesting deeper than this is refused (a hostile body can't overflow the
/// stack).
const MAX_DEPTH: usize = 256;

struct Parser<'a> {
    bytes: &'a [u8],
    text: &'a str,
    at: usize,
}

impl Parser<'_> {
    fn skip_whitespace(&mut self) {
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }

    fn eat(&mut self, literal: &str) -> bool {
        if self.text[self.at..].starts_with(literal) {
            self.at += literal.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self, depth: usize) -> Option<JsonValue> {
        if depth > MAX_DEPTH {
            return None;
        }
        self.skip_whitespace();
        match self.bytes.get(self.at)? {
            b'{' => self.object(depth),
            b'[' => self.array(depth),
            b'"' => self.string().map(JsonValue::String),
            b't' => self.eat("true").then_some(JsonValue::Bool(true)),
            b'f' => self.eat("false").then_some(JsonValue::Bool(false)),
            b'n' => self.eat("null").then_some(JsonValue::Null),
            _ => self.number(),
        }
    }

    fn object(&mut self, depth: usize) -> Option<JsonValue> {
        self.at += 1;
        let mut fields = Vec::new();
        self.skip_whitespace();
        if self.eat("}") {
            return Some(JsonValue::Object(fields));
        }
        loop {
            self.skip_whitespace();
            let key = self.string()?;
            self.skip_whitespace();
            if !self.eat(":") {
                return None;
            }
            fields.push((key, self.value(depth + 1)?));
            self.skip_whitespace();
            if self.eat("}") {
                return Some(JsonValue::Object(fields));
            }
            if !self.eat(",") {
                return None;
            }
        }
    }

    fn array(&mut self, depth: usize) -> Option<JsonValue> {
        self.at += 1;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.eat("]") {
            return Some(JsonValue::Array(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.skip_whitespace();
            if self.eat("]") {
                return Some(JsonValue::Array(items));
            }
            if !self.eat(",") {
                return None;
            }
        }
    }

    fn string(&mut self) -> Option<String> {
        if self.bytes.get(self.at) != Some(&b'"') {
            return None;
        }
        self.at += 1;
        let mut out = String::new();
        loop {
            let c = self.text[self.at..].chars().next()?;
            self.at += c.len_utf8();
            match c {
                '"' => return Some(out),
                '\\' => {
                    let escaped = self.text[self.at..].chars().next()?;
                    self.at += 1;
                    out.push(match escaped {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        'b' => '\u{8}',
                        'f' => '\u{c}',
                        'u' => self.unicode()?,
                        other => other,
                    });
                }
                c => out.push(c),
            }
        }
    }

    fn hex4(&mut self) -> Option<u32> {
        let hex = self.text.get(self.at..self.at + 4)?;
        self.at += 4;
        u32::from_str_radix(hex, 16).ok()
    }

    fn unicode(&mut self) -> Option<char> {
        let first = self.hex4()?;
        if (0xd800..0xdc00).contains(&first) && self.eat("\\u") {
            let low = self.hex4()?;
            return char::from_u32(0x10000 + ((first - 0xd800) << 10) + (low.checked_sub(0xdc00)?));
        }
        Some(char::from_u32(first).unwrap_or('\u{fffd}'))
    }

    fn number(&mut self) -> Option<JsonValue> {
        let start = self.at;
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| matches!(b, b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'))
        {
            self.at += 1;
        }
        self.text[start..self.at]
            .parse()
            .ok()
            .map(JsonValue::Number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_walks_paths() {
        let v = JsonValue::parse(
            r#" {"data": {"items": [{"id": 7, "name": "éé"}]}, "ok": true, "n": null} "#,
        )
        .unwrap();
        assert_eq!(v.at("data.items[0].id"), Some(&JsonValue::Number(7.0)));
        assert_eq!(v.at("data.items.0.name").unwrap().to_text(), "éé");
        assert_eq!(v.at("ok"), Some(&JsonValue::Bool(true)));
        assert_eq!(
            v.at("data['items'][0]").map(JsonValue::to_string),
            Some(r#"{"id":7,"name":"éé"}"#.into())
        );
        assert_eq!(v.at("missing"), None);
        assert_eq!(v.at(""), Some(&v));
    }

    #[test]
    fn refuses_what_is_not_json() {
        for bad in ["", "{", "[1,]", r#"{"a" 1}"#, "1 2", "tru"] {
            assert_eq!(JsonValue::parse(bad), None, "{bad}");
        }
        assert_eq!(JsonValue::parse(&"[".repeat(1000)), None);
    }

    #[test]
    fn surrogate_pairs_and_numbers() {
        assert_eq!(
            JsonValue::parse(r#""🦄""#),
            Some(JsonValue::String("🦄".into()))
        );
        assert_eq!(JsonValue::parse("-1.5e2"), Some(JsonValue::Number(-150.0)));
        assert_eq!(format_number(42.0), "42");
        assert_eq!(format_number(0.5), "0.5");
    }

    #[test]
    fn quotes_strings() {
        assert_eq!(quote("a\"b\\c\nd\u{1}"), r#""a\"b\\c\nd\u0001""#);
    }
}
