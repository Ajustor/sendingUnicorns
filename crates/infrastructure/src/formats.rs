//! Collection files: ours (written), plus what we read: our first format,
//! the files of sendingUnicorns 0.1–0.2 (Tauri) and Postman v2 collections.

mod legacy;
mod native;
mod postman;

use domain::{Collection, CollectionName, HistoryEntry, KeyValue, KeyValues};
use serde_json::Value;

/// Reads a collection in any format we know. `fallback_name` names it when
/// the file does not (or its name is not usable).
///
/// # Errors
///
/// When the text is not JSON or no collection format we know.
pub fn parse(text: &str, fallback_name: &str) -> Result<Collection, String> {
    let value: Value = serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
    let name_in_file = value
        .get("name")
        .or_else(|| value.pointer("/info/name"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let name = CollectionName::parse(name_in_file)
        .or_else(|_| CollectionName::parse(fallback_name))
        .or_else(|_| CollectionName::parse("Imported"))
        .map_err(|e| e.to_string())?;

    if value
        .get("format")
        .and_then(Value::as_str)
        .is_some_and(|f| f.starts_with("sending-unicorns/"))
    {
        return native::read(value, name);
    }
    if value
        .pointer("/info/schema")
        .and_then(Value::as_str)
        .is_some_and(|s| s.contains("postman"))
        || (value.get("info").is_some() && value.get("item").is_some())
    {
        return Ok(postman::read(&value, name));
    }
    if value.get("requests").is_some_and(Value::is_array) {
        return Ok(legacy::read(&value, name));
    }
    Err("not a collection (sendingUnicorns or Postman v2)".into())
}

/// Our format, indented (files are meant to be shared and diffed).
///
/// # Errors
///
/// Only if serialization fails, which plain data can't.
pub fn write(collection: &Collection) -> Result<String, String> {
    native::write(collection)
}

/// The history: entries with their request in our format.
///
/// # Errors
///
/// When the text is not such a list.
pub fn parse_history(text: &str) -> Result<Vec<HistoryEntry>, String> {
    native::parse_history(text)
}

/// # Errors
///
/// Only if serialization fails, which plain data can't.
pub fn write_history(entries: &[HistoryEntry]) -> Result<String, String> {
    native::write_history(entries)
}

/// Global variables: the rows of our format.
///
/// # Errors
///
/// When the text is not such a list.
pub fn parse_rows(text: &str) -> Result<KeyValues, String> {
    native::parse_rows(text)
}

/// # Errors
///
/// Only if serialization fails, which plain data can't.
pub fn write_rows(rows: &KeyValues) -> Result<String, String> {
    native::write_rows(rows)
}

fn array(value: Option<&Value>) -> &[Value] {
    value.and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

fn str_of(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str)
}

/// A string, number or boolean as text; `None` for null and the rest.
fn scalar(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn row(key: impl Into<String>, value: impl Into<String>, enabled: bool) -> KeyValue {
    KeyValue {
        enabled,
        ..KeyValue::new(key, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_fall_back_and_garbage_is_refused() {
        let c = parse(r#"{"name": "a/b", "requests": []}"#, "from file").unwrap();
        assert_eq!(c.name.as_str(), "from file");
        assert!(parse("[1, 2]", "x").is_err());
        assert!(parse("nope", "x").is_err());
    }
}
