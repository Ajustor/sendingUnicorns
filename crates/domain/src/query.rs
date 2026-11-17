//! The query string of an address, as a params table and back.
//!
//! Text is kept as typed: `{{variables}}` may sit anywhere, so nothing is
//! percent-encoded or decoded here (the HTTP adapter encodes what it sends).

use crate::key_value::{KeyValue, KeyValues};

/// Splits `https://host/path?a=1&b#frag` into the address without its query
/// (`https://host/path#frag`) and its params (`a=1`, `b=`).
pub fn split_query(url: &str) -> (String, Vec<KeyValue>) {
    let (before_fragment, fragment) = match url.find('#') {
        Some(i) => url.split_at(i),
        None => (url, ""),
    };
    let Some((base, query)) = before_fragment.split_once('?') else {
        return (url.to_owned(), Vec::new());
    };
    let params = query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| match pair.split_once('=') {
            Some((key, value)) => KeyValue::new(key, value),
            None => KeyValue::new(pair, ""),
        })
        .collect();
    (format!("{base}{fragment}"), params)
}

/// The address `base` (without a query) with the active `params` as its query.
pub fn join_query(base: &str, params: &KeyValues) -> String {
    let (path, fragment) = match base.find('#') {
        Some(i) => base.split_at(i),
        None => (base, ""),
    };
    let query: Vec<String> = params
        .active()
        .map(|kv| {
            if kv.value.is_empty() {
                kv.key.clone()
            } else {
                format!("{}={}", kv.key, kv.value)
            }
        })
        .collect();
    if query.is_empty() {
        format!("{path}{fragment}")
    } else {
        format!("{path}?{}{fragment}", query.join("&"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_the_query_and_keeps_the_fragment() {
        let (base, params) = split_query("https://h/p?a=1&b&&c={{x}}#top");
        assert_eq!(base, "https://h/p#top");
        assert_eq!(
            params,
            [
                KeyValue::new("a", "1"),
                KeyValue::new("b", ""),
                KeyValue::new("c", "{{x}}")
            ]
        );
    }

    #[test]
    fn an_address_without_query_has_no_params() {
        assert_eq!(
            split_query("{{base}}/users"),
            ("{{base}}/users".into(), vec![])
        );
    }

    #[test]
    fn joins_only_active_params() {
        let params = KeyValues(vec![
            KeyValue::new("a", "1"),
            KeyValue {
                enabled: false,
                ..KeyValue::new("off", "x")
            },
            KeyValue::new("flag", ""),
        ]);
        assert_eq!(
            join_query("https://h/p#f", &params),
            "https://h/p?a=1&flag#f"
        );
        assert_eq!(
            join_query("https://h/p", &KeyValues::default()),
            "https://h/p"
        );
    }

    #[test]
    fn split_then_join_is_stable() {
        let url = "http://localhost:8080/search?q=rust&page=2";
        let (base, params) = split_query(url);
        assert_eq!(join_query(&base, &KeyValues(params)), url);
    }
}
