//! sendingUnicorns 0.1–0.2 (Tauri).
//!
//! Rows were `[key, {"is_active": bool, "value": string|number|bool|null}]`;
//! bodies went through three shapes:
//!   0.1   `[[key, options]…]` (form only)
//!   0.2.0 `{"type": "Json"|"FormData"|"Null", "value": …}`
//!   0.2.3 `{"json": "…", "form_data": [[key, options]…]}`
//! Environment variables were `[[key, value]…]`.

use domain::{
    Body, BodyKind, Collection, CollectionName, Environment, HttpMethod, Item, KeyValue, KeyValues,
    Request,
};
use serde_json::Value;

use super::{array, row, scalar, str_of};

pub fn read(value: &Value, name: CollectionName) -> Collection {
    let items = array(value.get("requests"))
        .iter()
        .map(|r| {
            let mut request = Request::new(str_of(r.get("name")).unwrap_or("Request"));
            request.method = str_of(r.get("method"))
                .and_then(HttpMethod::parse)
                .unwrap_or_default();
            str_of(r.get("url"))
                .unwrap_or_default()
                .clone_into(&mut request.url);
            let options = r.get("options");
            request.headers = rows(options.and_then(|o| o.get("headers")));
            request.params = rows(options.and_then(|o| o.get("params")));
            if request.params.is_empty() {
                request.sync_params_from_url();
            } else {
                request.sync_path_params();
            }
            request.body = body(options.and_then(|o| o.get("body")));
            Item::Request(request)
        })
        .collect();
    let environments = array(value.get("environments"))
        .iter()
        .map(|e| Environment {
            name: str_of(e.get("name")).unwrap_or("Environment").to_owned(),
            variables: array(e.get("vars"))
                .iter()
                .filter_map(|pair| {
                    let pair = pair.as_array()?;
                    Some(KeyValue::new(
                        scalar(pair.first())?,
                        scalar(pair.get(1)).unwrap_or_default(),
                    ))
                })
                .collect(),
        })
        .collect();
    Collection {
        items,
        environments,
        ..Collection::new(name)
    }
}

fn rows(value: Option<&Value>) -> KeyValues {
    array(value)
        .iter()
        .filter_map(|pair| {
            let pair = pair.as_array()?;
            let key = scalar(pair.first())?;
            let options = pair.get(1);
            Some(row(
                key,
                scalar(options.and_then(|o| o.get("value"))).unwrap_or_default(),
                options
                    .and_then(|o| o.get("is_active"))
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
            ))
        })
        .collect()
}

fn body(value: Option<&Value>) -> Body {
    let mut body = Body::default();
    match value {
        Some(Value::Array(_)) => body.form = rows(value),
        Some(Value::Object(o)) if o.contains_key("type") => match str_of(o.get("type")) {
            Some("Json") => str_of(o.get("value"))
                .unwrap_or_default()
                .clone_into(&mut body.json),
            Some("FormData") => body.form = rows(o.get("value")),
            _ => {}
        },
        Some(Value::Object(o)) => {
            str_of(o.get("json"))
                .unwrap_or_default()
                .clone_into(&mut body.json);
            body.form = rows(o.get("form_data"));
        }
        _ => {}
    }
    // The kind was not saved: what was filled in is what was sent.
    body.kind = if !body.json.trim().is_empty() {
        BodyKind::Json
    } else if !body.form.is_empty() {
        BodyKind::Form
    } else {
        BodyKind::None
    };
    body
}

#[cfg(test)]
mod tests {
    use super::super::parse;
    use domain::{BodyKind, HttpMethod, Item, KeyValue};

    #[test]
    fn reads_sending_unicorns_0_2_files() {
        let text = r#"{
          "name": "old",
          "requests": [{
            "name": "login", "url": "{{host}}/login", "method": "post", "id": "x",
            "pre_request_script": null, "test": null,
            "options": {
              "body": {"json": "{\"u\":1}", "form_data": [["k", {"is_active": true, "value": 3}]]},
              "params": [["page", {"is_active": false, "value": "2"}]],
              "headers": [["Authorization", {"is_active": true, "value": "Bearer {{t}}"}]]
            }
          }, {
            "name": "first", "url": "http://h?a=1", "method": "get",
            "options": {"body": [["f", {"is_active": true, "value": true}]]}
          }, {
            "name": "typed", "url": "http://h", "method": "put",
            "options": {"body": {"type": "Json", "value": "[1]"}}
          }],
          "environments": [{"name": "dev", "id": "dev", "vars": [["host", "http://localhost"]]}]
        }"#;
        let c = parse(text, "fallback").unwrap();
        assert_eq!(c.name.as_str(), "old");
        let requests = c.requests();
        let login = requests[0];
        assert_eq!(login.method, HttpMethod::Post);
        assert_eq!(login.body.kind, BodyKind::Json);
        assert_eq!(login.body.form.0, [KeyValue::new("k", "3")]);
        assert!(!login.params.0[0].enabled);
        assert_eq!(
            login.headers.find_ignore_case("authorization"),
            Some("Bearer {{t}}")
        );
        let first = requests[1];
        assert_eq!(first.body.kind, BodyKind::Form);
        assert_eq!(first.body.form.0, [KeyValue::new("f", "true")]);
        assert_eq!(first.params.0, [KeyValue::new("a", "1")]);
        assert_eq!(requests[2].body.json, "[1]");
        assert!(matches!(c.items[0], Item::Request(_)));
        assert_eq!(
            c.environments[0].variables.0,
            [KeyValue::new("host", "http://localhost")]
        );
    }
}
