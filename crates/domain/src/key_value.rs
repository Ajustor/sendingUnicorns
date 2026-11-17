//! The rows of the params, headers, form and variables tables.

/// One row: a key, its value, and whether it is sent.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    pub enabled: bool,
    /// Shown masked (tokens, passwords); sent like any other.
    pub secret: bool,
}

impl KeyValue {
    /// An enabled row.
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            enabled: true,
            secret: false,
        }
    }

    /// A row that counts: enabled, with a key.
    pub fn is_active(&self) -> bool {
        self.enabled && !self.key.trim().is_empty()
    }
}

/// An ordered table of rows; keys may repeat (`?tag=a&tag=b`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct KeyValues(pub Vec<KeyValue>);

impl KeyValues {
    /// The rows that are sent, in order.
    pub fn active(&self) -> impl Iterator<Item = &KeyValue> {
        self.0.iter().filter(|kv| kv.is_active())
    }

    /// The value of the last active row named `key` (case-insensitive, as
    /// HTTP header names are).
    pub fn find_ignore_case(&self, key: &str) -> Option<&str> {
        self.active()
            .filter(|kv| kv.key.trim().eq_ignore_ascii_case(key))
            .last()
            .map(|kv| kv.value.as_str())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl FromIterator<KeyValue> for KeyValues {
    fn from_iter<T: IntoIterator<Item = KeyValue>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_enabled_rows_with_a_key_are_active() {
        let rows = KeyValues(vec![
            KeyValue::new("a", "1"),
            KeyValue {
                enabled: false,
                ..KeyValue::new("b", "2")
            },
            KeyValue::new("  ", "3"),
        ]);
        let keys: Vec<_> = rows.active().map(|kv| kv.key.as_str()).collect();
        assert_eq!(keys, ["a"]);
    }

    #[test]
    fn header_lookup_ignores_case_and_takes_the_last() {
        let rows = KeyValues(vec![
            KeyValue::new("Content-Type", "text/plain"),
            KeyValue::new("content-type", "application/json"),
        ]);
        assert_eq!(
            rows.find_ignore_case("CONTENT-TYPE"),
            Some("application/json")
        );
        assert_eq!(rows.find_ignore_case("accept"), None);
    }
}
