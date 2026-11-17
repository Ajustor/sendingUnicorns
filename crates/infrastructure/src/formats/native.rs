//! Our format. Version 2 (written) holds folders, auth, variables and
//! assertions; version 1 (flat `requests`) is still read.

use domain::{
    AssertOp, Assertion, Auth, Body, BodyKind, Collection, CollectionName, Environment, Folder,
    HistoryEntry, HttpMethod, Item, KeyLocation, KeyValue, KeyValues, Part, Request,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const FORMAT: &str = "sending-unicorns/2";

#[derive(Debug, Serialize, Deserialize)]
struct FileDto {
    format: String,
    name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    docs: String,
    #[serde(default, skip_serializing_if = "AuthDto::is_none")]
    auth: AuthDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    headers: Vec<RowDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    variables: Vec<RowDto>,
    #[serde(default)]
    items: Vec<ItemDto>,
    /// Version 1: no folders.
    #[serde(default, skip_serializing)]
    requests: Vec<RequestDto>,
    #[serde(default)]
    environments: Vec<EnvironmentDto>,
}

// Read once and dropped: the size difference costs nothing (R-VER-10).
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ItemDto {
    Request(RequestDto),
    Folder(FolderDto),
}

#[derive(Debug, Serialize, Deserialize)]
struct FolderDto {
    name: String,
    #[serde(default, skip_serializing_if = "AuthDto::is_inherit")]
    auth: AuthDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    headers: Vec<RowDto>,
    #[serde(default)]
    items: Vec<ItemDto>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RequestDto {
    name: String,
    method: String,
    url: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    params: Vec<RowDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    path_params: Vec<RowDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    headers: Vec<RowDto>,
    #[serde(default)]
    body: BodyDto,
    #[serde(default, skip_serializing_if = "AuthDto::is_inherit")]
    auth: AuthDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    vars: Vec<RowDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    post_vars: Vec<RowDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    assertions: Vec<AssertionDto>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    docs: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct BodyDto {
    #[serde(default)]
    kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    json: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    form: Vec<RowDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    multipart: Vec<PartDto>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    text: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    xml: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    graphql_query: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    graphql_variables: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    file: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RowDto {
    key: String,
    #[serde(default)]
    value: String,
    #[serde(default = "yes")]
    enabled: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    secret: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct PartDto {
    key: String,
    #[serde(default)]
    value: String,
    #[serde(default, skip_serializing_if = "is_false")]
    file: bool,
    #[serde(default = "yes")]
    enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct AssertionDto {
    expr: String,
    op: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    value: String,
    #[serde(default = "yes")]
    enabled: bool,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum AuthDto {
    #[default]
    Inherit,
    None,
    Bearer {
        token: String,
    },
    Basic {
        username: String,
        #[serde(default)]
        password: String,
    },
    ApiKey {
        key: String,
        #[serde(default)]
        value: String,
        /// `header` or `query`.
        #[serde(default, rename = "in")]
        location: String,
    },
}

impl AuthDto {
    fn is_inherit(&self) -> bool {
        *self == Self::Inherit
    }

    fn is_none(&self) -> bool {
        *self == Self::None
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct EnvironmentDto {
    name: String,
    #[serde(default)]
    variables: Vec<RowDto>,
}

fn yes() -> bool {
    true
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde's skip_serializing_if signature
fn is_false(b: &bool) -> bool {
    !*b
}

pub fn read(value: Value, name: CollectionName) -> Result<Collection, String> {
    let file: FileDto = serde_json::from_value(value).map_err(|e| e.to_string())?;
    let mut items: Vec<Item> = file.items.into_iter().map(item).collect();
    items.extend(file.requests.into_iter().map(|r| Item::Request(request(r))));
    Ok(Collection {
        name,
        items,
        environments: file
            .environments
            .into_iter()
            .map(|e| Environment {
                name: e.name,
                variables: rows(e.variables),
            })
            .collect(),
        variables: rows(file.variables),
        // A collection has nothing to inherit from.
        auth: match auth(file.auth) {
            Auth::Inherit => Auth::None,
            other => other,
        },
        headers: rows(file.headers),
        docs: file.docs,
    })
}

pub fn write(c: &Collection) -> Result<String, String> {
    let file = FileDto {
        format: FORMAT.into(),
        name: c.name.to_string(),
        docs: c.docs.clone(),
        auth: auth_dto(&c.auth),
        headers: row_dtos(&c.headers),
        variables: row_dtos(&c.variables),
        items: c.items.iter().map(item_dto).collect(),
        requests: Vec::new(),
        environments: c
            .environments
            .iter()
            .map(|e| EnvironmentDto {
                name: e.name.clone(),
                variables: row_dtos(&e.variables),
            })
            .collect(),
    };
    serde_json::to_string_pretty(&file).map_err(|e| e.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
struct HistoryDto {
    at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    status: Option<u16>,
    #[serde(default)]
    elapsed_ms: u64,
    request: RequestDto,
}

pub fn parse_history(text: &str) -> Result<Vec<HistoryEntry>, String> {
    let dtos: Vec<HistoryDto> = serde_json::from_str(text).map_err(|e| e.to_string())?;
    Ok(dtos
        .into_iter()
        .map(|h| HistoryEntry {
            at: h.at,
            request: request(h.request),
            status: h.status,
            elapsed_ms: h.elapsed_ms,
        })
        .collect())
}

/// Compact: the history is not meant to be read by people.
pub fn write_history(entries: &[HistoryEntry]) -> Result<String, String> {
    let dtos: Vec<HistoryDto> = entries
        .iter()
        .map(|h| HistoryDto {
            at: h.at,
            status: h.status,
            elapsed_ms: h.elapsed_ms,
            request: request_dto(&h.request),
        })
        .collect();
    serde_json::to_string(&dtos).map_err(|e| e.to_string())
}

pub fn parse_rows(text: &str) -> Result<KeyValues, String> {
    let dtos: Vec<RowDto> = serde_json::from_str(text).map_err(|e| e.to_string())?;
    Ok(rows(dtos))
}

pub fn write_rows(r: &KeyValues) -> Result<String, String> {
    serde_json::to_string_pretty(&row_dtos(r)).map_err(|e| e.to_string())
}

fn item(dto: ItemDto) -> Item {
    match dto {
        ItemDto::Request(r) => Item::Request(request(r)),
        ItemDto::Folder(f) => Item::Folder(Folder {
            auth: auth(f.auth),
            headers: rows(f.headers),
            items: f.items.into_iter().map(item).collect(),
            ..Folder::new(f.name)
        }),
    }
}

fn item_dto(item: &Item) -> ItemDto {
    match item {
        Item::Request(r) => ItemDto::Request(request_dto(r)),
        Item::Folder(f) => ItemDto::Folder(FolderDto {
            name: f.name.clone(),
            auth: auth_dto(&f.auth),
            headers: row_dtos(&f.headers),
            items: f.items.iter().map(item_dto).collect(),
        }),
    }
}

fn request(dto: RequestDto) -> Request {
    let mut r = Request::new(dto.name);
    r.method = HttpMethod::parse(&dto.method).unwrap_or_default();
    r.url = dto.url;
    r.params = rows(dto.params);
    r.path_params = rows(dto.path_params);
    // Files of version 1 had no path params: rows for the `:names`.
    r.sync_path_params();
    r.headers = rows(dto.headers);
    let b = dto.body;
    r.body = Body {
        kind: match b.kind.as_str() {
            "json" => BodyKind::Json,
            "form" => BodyKind::Form,
            "multipart" => BodyKind::Multipart,
            "text" => BodyKind::Text,
            "xml" => BodyKind::Xml,
            "graphql" => BodyKind::GraphQl,
            "file" => BodyKind::File,
            _ => BodyKind::None,
        },
        json: b.json,
        form: rows(b.form),
        multipart: b
            .multipart
            .into_iter()
            .map(|p| Part {
                key: p.key,
                value: p.value,
                is_file: p.file,
                enabled: p.enabled,
            })
            .collect(),
        text: b.text,
        xml: b.xml,
        graphql_query: b.graphql_query,
        graphql_variables: b.graphql_variables,
        file: b.file,
    };
    r.auth = auth(dto.auth);
    r.vars = rows(dto.vars);
    r.post_vars = rows(dto.post_vars);
    r.assertions = dto
        .assertions
        .into_iter()
        .map(|a| Assertion {
            expr: a.expr,
            op: AssertOp::parse(&a.op).unwrap_or_default(),
            value: a.value,
            enabled: a.enabled,
        })
        .collect();
    r.docs = dto.docs;
    r
}

fn request_dto(r: &Request) -> RequestDto {
    let b = &r.body;
    RequestDto {
        name: r.name.clone(),
        method: r.method.as_str().into(),
        url: r.url.clone(),
        params: row_dtos(&r.params),
        path_params: row_dtos(&r.path_params),
        headers: row_dtos(&r.headers),
        body: BodyDto {
            kind: match b.kind {
                BodyKind::None => "none",
                BodyKind::Json => "json",
                BodyKind::Form => "form",
                BodyKind::Multipart => "multipart",
                BodyKind::Text => "text",
                BodyKind::Xml => "xml",
                BodyKind::GraphQl => "graphql",
                BodyKind::File => "file",
            }
            .into(),
            json: b.json.clone(),
            form: row_dtos(&b.form),
            multipart: b
                .multipart
                .iter()
                .map(|p| PartDto {
                    key: p.key.clone(),
                    value: p.value.clone(),
                    file: p.is_file,
                    enabled: p.enabled,
                })
                .collect(),
            text: b.text.clone(),
            xml: b.xml.clone(),
            graphql_query: b.graphql_query.clone(),
            graphql_variables: b.graphql_variables.clone(),
            file: b.file.clone(),
        },
        auth: auth_dto(&r.auth),
        vars: row_dtos(&r.vars),
        post_vars: row_dtos(&r.post_vars),
        assertions: r
            .assertions
            .iter()
            .map(|a| AssertionDto {
                expr: a.expr.clone(),
                op: a.op.as_str().into(),
                value: a.value.clone(),
                enabled: a.enabled,
            })
            .collect(),
        docs: r.docs.clone(),
    }
}

fn auth(dto: AuthDto) -> Auth {
    match dto {
        AuthDto::Inherit => Auth::Inherit,
        AuthDto::None => Auth::None,
        AuthDto::Bearer { token } => Auth::Bearer { token },
        AuthDto::Basic { username, password } => Auth::Basic { username, password },
        AuthDto::ApiKey {
            key,
            value,
            location,
        } => Auth::ApiKey {
            key,
            value,
            location: if location == "query" {
                KeyLocation::Query
            } else {
                KeyLocation::Header
            },
        },
    }
}

fn auth_dto(a: &Auth) -> AuthDto {
    match a {
        Auth::Inherit => AuthDto::Inherit,
        Auth::None => AuthDto::None,
        Auth::Bearer { token } => AuthDto::Bearer {
            token: token.clone(),
        },
        Auth::Basic { username, password } => AuthDto::Basic {
            username: username.clone(),
            password: password.clone(),
        },
        Auth::ApiKey {
            key,
            value,
            location,
        } => AuthDto::ApiKey {
            key: key.clone(),
            value: value.clone(),
            location: match location {
                KeyLocation::Header => "header",
                KeyLocation::Query => "query",
            }
            .into(),
        },
    }
}

fn rows(dtos: Vec<RowDto>) -> KeyValues {
    dtos.into_iter()
        .map(|r| KeyValue {
            key: r.key,
            value: r.value,
            enabled: r.enabled,
            secret: r.secret,
        })
        .collect()
}

fn row_dtos(rows: &KeyValues) -> Vec<RowDto> {
    rows.0
        .iter()
        .map(|kv| RowDto {
            key: kv.key.clone(),
            value: kv.value.clone(),
            enabled: kv.enabled,
            secret: kv.secret,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::{parse, write};
    use super::*;

    fn full() -> Collection {
        let mut c = Collection::new(CollectionName::parse("api").unwrap());
        c.auth = Auth::Bearer {
            token: "{{token}}".into(),
        };
        c.headers.0.push(KeyValue::new("X-App", "su"));
        c.variables.0.push(KeyValue {
            secret: true,
            ..KeyValue::new("token", "s3cret")
        });
        c.docs = "# API".into();
        let mut r = Request::new("create");
        r.method = HttpMethod::Post;
        r.url = "{{base}}/users/:id?x=1".into();
        r.sync_params_from_url();
        r.path_params.0[0].value = "7".into();
        r.params.0.push(KeyValue {
            enabled: false,
            ..KeyValue::new("off", "1")
        });
        r.body = Body {
            kind: BodyKind::Multipart,
            json: "{}".into(),
            multipart: vec![Part {
                key: "f".into(),
                value: "/a.png".into(),
                is_file: true,
                enabled: true,
            }],
            graphql_query: "{ a }".into(),
            ..Body::default()
        };
        r.auth = Auth::ApiKey {
            key: "k".into(),
            value: "v".into(),
            location: KeyLocation::Query,
        };
        r.post_vars.0.push(KeyValue::new("id", "res.body.id"));
        r.assertions
            .push(Assertion::new("res.status", AssertOp::Lt, "300"));
        r.docs = "Creates".into();
        let mut folder = Folder::new("users");
        folder.auth = Auth::None;
        folder.items.push(Item::Request(r));
        c.items.push(Item::Folder(folder));
        c.items.push(Item::Request(Request::new("ping")));
        c.add_environment("dev").unwrap();
        c.environment_mut("dev")
            .unwrap()
            .variables
            .0
            .push(KeyValue::new("base", "http://h"));
        c
    }

    /// Equal but for the ids, which every read makes anew.
    fn same(a: &Collection, b: &Collection) {
        assert_eq!(write(a).unwrap(), write(b).unwrap());
    }

    #[test]
    fn version_2_round_trips() {
        let c = full();
        let text = write(&c).unwrap();
        assert!(text.contains("\"format\": \"sending-unicorns/2\""));
        let back = parse(&text, "ignored").unwrap();
        same(&back, &c);
        let created = back.requests()[0];
        assert_eq!(created.path_params.0[0].value, "7");
        assert!(back.variables.0[0].secret);
        assert!(matches!(&back.items[0], Item::Folder(f) if f.auth == Auth::None));
    }

    #[test]
    fn reads_version_1() {
        let text = r#"{"format": "sending-unicorns/1", "name": "old", "requests": [
            {"name": "get", "method": "GET", "url": "http://h/:id", "body": {"kind": "json", "json": "{}"}}
        ], "environments": []}"#;
        let c = parse(text, "x").unwrap();
        let r = c.requests()[0];
        assert_eq!(r.body.kind, BodyKind::Json);
        assert_eq!(r.path_params.0, [KeyValue::new("id", "")]);
        assert_eq!(c.auth, Auth::None);
    }

    #[test]
    fn rows_round_trip() {
        let rows = KeyValues(vec![KeyValue {
            secret: true,
            ..KeyValue::new("a", "b")
        }]);
        assert_eq!(parse_rows(&write_rows(&rows).unwrap()).unwrap(), rows);
    }
}
