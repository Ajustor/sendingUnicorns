//! Postman v2.0 / v2.1 collections: folders, auth at every level, every
//! body mode, path variables, collection variables. Scripts (`event`) are
//! left out: use post-response variables and assertions instead.

use domain::{
    Auth, BodyKind, Collection, CollectionName, Folder, HttpMethod, Item, KeyLocation, KeyValues,
    Part, Request,
};
use serde_json::Value;

use super::{array, row, scalar, str_of};

pub fn read(value: &Value, name: CollectionName) -> Collection {
    Collection {
        items: items(array(value.get("item"))),
        variables: rows(value.get("variable")),
        auth: match value.get("auth").map(auth) {
            Some(Auth::Inherit) | None => Auth::None,
            Some(other) => other,
        },
        docs: description(value.pointer("/info/description")),
        ..Collection::new(name)
    }
}

fn items(values: &[Value]) -> Vec<Item> {
    values
        .iter()
        .filter_map(|item| {
            let name = str_of(item.get("name")).unwrap_or("Request");
            if let Some(children) = item.get("item").and_then(Value::as_array) {
                return Some(Item::Folder(Folder {
                    items: items(children),
                    auth: item.get("auth").map_or(Auth::Inherit, auth),
                    ..Folder::new(name)
                }));
            }
            let mut r = request(item.get("request")?, name);
            r.docs = description(item.pointer("/request/description"));
            Some(Item::Request(r))
        })
        .collect()
}

/// Descriptions are a string or `{"content": …}`.
fn description(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(s)) => s.clone(),
        Some(v) => str_of(v.get("content")).unwrap_or_default().to_owned(),
        None => String::new(),
    }
}

fn request(r: &Value, name: &str) -> Request {
    let mut request = Request::new(name);
    // v2 allows a bare string for a GET.
    if let Some(url) = r.as_str() {
        url.clone_into(&mut request.url);
        request.sync_params_from_url();
        return request;
    }
    request.method = str_of(r.get("method"))
        .and_then(HttpMethod::parse)
        .unwrap_or_default();
    request.url = match r.get("url") {
        Some(Value::String(s)) => s.clone(),
        Some(u) => str_of(u.get("raw")).unwrap_or_default().to_owned(),
        None => String::new(),
    };
    request.sync_params_from_url();
    let path_values = rows(r.pointer("/url/variable"));
    for param in &mut request.path_params.0 {
        if let Some(v) = path_values.0.iter().find(|v| v.key == param.key) {
            param.value.clone_from(&v.value);
        }
    }
    request.headers = rows(r.get("header"));
    request.auth = r.get("auth").map_or(Auth::Inherit, auth);
    body(&mut request, r.get("body"));
    request
}

fn body(request: &mut Request, body: Option<&Value>) {
    let Some(body) = body else {
        return;
    };
    let b = &mut request.body;
    match str_of(body.get("mode")) {
        Some("raw") => {
            let raw = str_of(body.get("raw")).unwrap_or_default().to_owned();
            let language = str_of(body.pointer("/options/raw/language")).unwrap_or_default();
            let content_type = request
                .headers
                .find_ignore_case("content-type")
                .unwrap_or_default()
                .to_ascii_lowercase();
            if language == "json" || content_type.contains("json") {
                b.kind = BodyKind::Json;
                b.json = raw;
            } else if language == "xml" || content_type.contains("xml") {
                b.kind = BodyKind::Xml;
                b.xml = raw;
            } else {
                b.kind = BodyKind::Text;
                b.text = raw;
            }
        }
        Some("urlencoded") => {
            b.kind = BodyKind::Form;
            b.form = rows(body.get("urlencoded"));
        }
        Some("formdata") => {
            b.kind = BodyKind::Multipart;
            b.multipart = array(body.get("formdata"))
                .iter()
                .filter_map(|p| {
                    let is_file = str_of(p.get("type")) == Some("file");
                    let value = if is_file {
                        match p.get("src") {
                            Some(Value::Array(srcs)) => scalar(srcs.first()),
                            other => scalar(other),
                        }
                    } else {
                        scalar(p.get("value"))
                    };
                    Some(Part {
                        key: str_of(p.get("key"))?.to_owned(),
                        value: value.unwrap_or_default(),
                        is_file,
                        enabled: !p.get("disabled").and_then(Value::as_bool).unwrap_or(false),
                    })
                })
                .collect();
        }
        Some("graphql") => {
            b.kind = BodyKind::GraphQl;
            str_of(body.pointer("/graphql/query"))
                .unwrap_or_default()
                .clone_into(&mut b.graphql_query);
            b.graphql_variables = match body.pointer("/graphql/variables") {
                Some(Value::String(s)) => s.clone(),
                Some(Value::Null) | None => String::new(),
                Some(v) => v.to_string(),
            };
        }
        Some("file") => {
            b.kind = BodyKind::File;
            b.file = scalar(body.pointer("/file/src")).unwrap_or_default();
        }
        _ => {}
    }
}

/// `{"type": "bearer", "bearer": [{"key": "token", "value": …}]}`.
fn auth(value: &Value) -> Auth {
    let field = |kind: &str, name: &str| {
        array(value.get(kind))
            .iter()
            .find(|v| str_of(v.get("key")) == Some(name))
            .and_then(|v| scalar(v.get("value")))
            .unwrap_or_default()
    };
    match str_of(value.get("type")) {
        Some("noauth") => Auth::None,
        Some("bearer") => Auth::Bearer {
            token: field("bearer", "token"),
        },
        Some("basic") => Auth::Basic {
            username: field("basic", "username"),
            password: field("basic", "password"),
        },
        Some("apikey") => Auth::ApiKey {
            key: field("apikey", "key"),
            value: field("apikey", "value"),
            location: if field("apikey", "in") == "query" {
                KeyLocation::Query
            } else {
                KeyLocation::Header
            },
        },
        _ => Auth::Inherit,
    }
}

fn rows(value: Option<&Value>) -> KeyValues {
    array(value)
        .iter()
        .filter_map(|v| {
            Some(row(
                str_of(v.get("key"))?,
                scalar(v.get("value")).unwrap_or_default(),
                !v.get("disabled").and_then(Value::as_bool).unwrap_or(false),
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::parse;
    use domain::{Auth, BodyKind, Item, KeyValue};

    #[test]
    fn reads_postman_collections() {
        let text = r#"{
          "info": {"name": "Shop", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json", "description": "The shop"},
          "auth": {"type": "bearer", "bearer": [{"key": "token", "value": "{{token}}"}]},
          "item": [
            {"name": "Users", "auth": {"type": "basic", "basic": [{"key": "username", "value": "ada"}, {"key": "password", "value": "pw"}]}, "item": [
              {"name": "Get", "request": {"method": "GET", "url": {"raw": "{{base}}/users/:id?limit=5", "variable": [{"key": "id", "value": "7"}]}}}
            ]},
            {"name": "Create", "request": {"method": "POST", "url": "{{base}}/users", "auth": {"type": "noauth"},
              "header": [{"key": "X-Off", "value": "1", "disabled": true}],
              "body": {"mode": "raw", "raw": "{\"a\":1}", "options": {"raw": {"language": "json"}}}}},
            {"name": "Upload", "request": {"method": "POST", "url": "{{base}}/up",
              "body": {"mode": "formdata", "formdata": [{"key": "f", "type": "file", "src": ["/tmp/a.png"]}, {"key": "n", "value": "x"}]}}},
            {"name": "Query", "request": {"method": "POST", "url": "{{base}}/graphql",
              "body": {"mode": "graphql", "graphql": {"query": "{ me { id } }", "variables": "{\"a\":1}"}}}},
            {"name": "Login", "request": {"method": "POST", "url": "{{base}}/login",
              "body": {"mode": "urlencoded", "urlencoded": [{"key": "user", "value": "me"}]}}}
          ],
          "variable": [{"key": "base", "value": "https://shop.test"}]
        }"#;
        let c = parse(text, "x").unwrap();
        assert_eq!(c.name.as_str(), "Shop");
        assert_eq!(c.docs, "The shop");
        assert_eq!(
            c.auth,
            Auth::Bearer {
                token: "{{token}}".into()
            }
        );
        assert_eq!(c.variables.0, [KeyValue::new("base", "https://shop.test")]);
        let Item::Folder(users) = &c.items[0] else {
            panic!("a folder");
        };
        assert!(matches!(users.auth, Auth::Basic { .. }));
        let requests = c.requests();
        let names: Vec<_> = requests.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["Get", "Create", "Upload", "Query", "Login"]);
        assert_eq!(requests[0].params.0, [KeyValue::new("limit", "5")]);
        assert_eq!(requests[0].path_params.0, [KeyValue::new("id", "7")]);
        assert_eq!(requests[0].auth, Auth::Inherit);
        assert_eq!(requests[1].auth, Auth::None);
        assert!(!requests[1].headers.0[0].enabled);
        assert_eq!(requests[1].body.kind, BodyKind::Json);
        assert!(requests[2].body.multipart[0].is_file);
        assert_eq!(requests[2].body.multipart[0].value, "/tmp/a.png");
        assert_eq!(requests[3].body.kind, BodyKind::GraphQl);
        assert_eq!(requests[3].body.graphql_variables, "{\"a\":1}");
        assert_eq!(requests[4].body.form.0, [KeyValue::new("user", "me")]);
    }
}
