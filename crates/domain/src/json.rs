//! JSON for the eyes: indentation and token kinds for colours. Lenient on
//! purpose: bodies may hold `{{variables}}` or be half-typed.

use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonToken {
    /// `{ } [ ] : ,`
    Punct,
    /// A string followed by `:`.
    Key,
    String,
    Number,
    /// `true`, `false`, `null`.
    Literal,
    /// `{{name}}`.
    Placeholder,
    Whitespace,
    Other,
}

/// Cuts `text` into tokens covering it entirely, in order.
pub fn tokenize_json(text: &str) -> Vec<(JsonToken, Range<usize>)> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        let kind = match bytes[i] {
            b'{' if bytes.get(i + 1) == Some(&b'{') => {
                i = text[i..].find("}}").map_or(bytes.len(), |end| i + end + 2);
                JsonToken::Placeholder
            }
            b'{' | b'}' | b'[' | b']' | b':' | b',' => {
                i += 1;
                JsonToken::Punct
            }
            b'"' => {
                i = string_end(bytes, i);
                JsonToken::String
            }
            b if b.is_ascii_whitespace() => {
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                JsonToken::Whitespace
            }
            b'-' | b'0'..=b'9' => {
                while i < bytes.len()
                    && matches!(bytes[i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
                {
                    i += 1;
                }
                JsonToken::Number
            }
            _ => {
                while i < bytes.len() && bytes[i].is_ascii_alphanumeric() {
                    i += 1;
                }
                if i == start {
                    // One character, whole (it may be multi-byte).
                    i += text[i..].chars().next().map_or(1, char::len_utf8);
                }
                match &text[start..i] {
                    "true" | "false" | "null" => JsonToken::Literal,
                    _ => JsonToken::Other,
                }
            }
        };
        tokens.push((kind, start..i));
    }
    mark_keys(text, &mut tokens);
    tokens
}

/// The index just past the string starting at `start` (its closing quote),
/// or the end of the text.
fn string_end(bytes: &[u8], start: usize) -> usize {
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => return i + 1,
            _ => i += 1,
        }
    }
    bytes.len()
}

fn mark_keys(text: &str, tokens: &mut [(JsonToken, Range<usize>)]) {
    for i in 0..tokens.len() {
        if tokens[i].0 != JsonToken::String {
            continue;
        }
        let next = tokens[i + 1..]
            .iter()
            .find(|(kind, _)| *kind != JsonToken::Whitespace);
        if next.is_some_and(|(kind, r)| *kind == JsonToken::Punct && &text[r.clone()] == ":") {
            tokens[i].0 = JsonToken::Key;
        }
    }
}

/// `text` indented with two spaces; `None` when its brackets don't match
/// (not JSON enough to be reshaped).
pub fn pretty_json(text: &str) -> Option<String> {
    let mut out = String::with_capacity(text.len() + text.len() / 2);
    let mut stack = Vec::new();
    let tokens = tokenize_json(text);
    let mut iter = tokens
        .iter()
        .filter(|(k, _)| *k != JsonToken::Whitespace)
        .peekable();
    while let Some((kind, range)) = iter.next() {
        let piece = &text[range.clone()];
        if *kind != JsonToken::Punct {
            out.push_str(piece);
            continue;
        }
        match piece {
            "{" | "[" => {
                let close = if piece == "{" { "}" } else { "]" };
                stack.push(close);
                out.push_str(piece);
                if iter.peek().is_some_and(|(_, r)| &text[r.clone()] == close) {
                    // `{}` and `[]` stay on one line.
                    iter.next();
                    stack.pop();
                    out.push_str(close);
                } else {
                    newline(&mut out, stack.len());
                }
            }
            "}" | "]" => {
                if stack.pop() != Some(piece) {
                    return None;
                }
                newline(&mut out, stack.len());
                out.push_str(piece);
            }
            "," => {
                out.push(',');
                newline(&mut out, stack.len());
            }
            _ => out.push_str(": "),
        }
    }
    stack.is_empty().then_some(out)
}

fn newline(out: &mut String, depth: usize) {
    out.push('\n');
    out.extend(std::iter::repeat_n("  ", depth));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indents() {
        let pretty = pretty_json(r#"{"a":[1,2],"b":{},"c":"x,{y}"}"#).unwrap();
        assert_eq!(
            pretty,
            "{\n  \"a\": [\n    1,\n    2\n  ],\n  \"b\": {},\n  \"c\": \"x,{y}\"\n}"
        );
    }

    #[test]
    fn keeps_placeholders() {
        assert_eq!(
            pretty_json(r#"{"id":{{id}}}"#).unwrap(),
            "{\n  \"id\": {{id}}\n}"
        );
    }

    #[test]
    fn refuses_unbalanced_text() {
        assert_eq!(pretty_json(r#"{"a":[1}"#), None);
        assert_eq!(pretty_json("{"), None);
    }

    #[test]
    fn tokens_cover_the_text_and_find_keys() {
        use JsonToken::{Key, Literal, Number};
        let text = r#"{ "k": "v", "n": -1.5e3, "t": true, "é": null }"#;
        let tokens = tokenize_json(text);
        let rebuilt: String = tokens.iter().map(|(_, r)| &text[r.clone()]).collect();
        assert_eq!(rebuilt, text);
        let kinds: Vec<_> = tokens
            .iter()
            .filter(|(k, _)| !matches!(k, JsonToken::Whitespace | JsonToken::Punct))
            .map(|(k, _)| *k)
            .collect();
        assert_eq!(
            kinds,
            [
                Key,
                JsonToken::String,
                Key,
                Number,
                Key,
                Literal,
                Key,
                Literal
            ]
        );
    }

    #[test]
    fn unterminated_strings_end_the_text() {
        let tokens = tokenize_json(r#"["abc"#);
        assert_eq!(tokens.last().unwrap(), &(JsonToken::String, 1..5));
    }
}
