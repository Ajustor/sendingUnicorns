//! From what the user wrote to what goes on the wire: variables resolved,
//! disabled rows dropped, auth and inherited headers applied.

use std::collections::BTreeSet;
use std::fmt;

use crate::auth::{Auth, AuthEffect};
use crate::json_value::quote;
use crate::key_value::KeyValue;
use crate::request::{BodyKind, HttpMethod, Request, fill_path_variables};
use crate::template::Variables;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartValue {
    Text(String),
    /// A file's path, read by the HTTP adapter.
    File(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparedBody {
    None,
    Json(String),
    /// Pairs, encoded by the HTTP adapter.
    Form(Vec<(String, String)>),
    Multipart(Vec<(String, PartValue)>),
    /// Text with its default `Content-Type`.
    Text {
        content_type: &'static str,
        text: String,
    },
    /// A file's path, sent as is.
    File(String),
}

/// A request ready to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedRequest {
    pub method: HttpMethod,
    /// Starts with `http://` or `https://`.
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: PreparedBody,
    /// `{{names}}` used but not defined: sent as written.
    pub missing_variables: BTreeSet<String>,
}

impl PreparedRequest {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .rev()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// What a request gets from its folders and collection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Inherited {
    /// Used when the request's own auth is [`Auth::Inherit`].
    pub auth: Auth,
    /// Sent before the request's own headers.
    pub headers: Vec<KeyValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PrepareError {
    EmptyUrl,
    /// `ftp://…`: only HTTP is sent.
    UnsupportedScheme(String),
    /// The File body has no file chosen.
    NoFile,
}

impl fmt::Display for PrepareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyUrl => f.write_str("Enter an address to send the request to."),
            Self::UnsupportedScheme(s) => write!(f, "{s}:// is not HTTP: use http:// or https://."),
            Self::NoFile => f.write_str("Choose the file to send as the body."),
        }
    }
}

impl std::error::Error for PrepareError {}

/// # Errors
///
/// When the address, once its variables are resolved, is empty or not HTTP,
/// or a File body has no file.
pub fn prepare(
    request: &Request,
    inherited: &Inherited,
    variables: &Variables,
) -> Result<PreparedRequest, PrepareError> {
    let mut missing = BTreeSet::new();
    let mut render = |text: &str| {
        let (out, unknown) = variables.render(text);
        missing.extend(unknown);
        out
    };

    let url = render(request.url.trim());
    let path_values: Vec<(String, String)> = request
        .path_params
        .active()
        .map(|kv| (kv.key.clone(), render(&kv.value)))
        .collect();
    let url = fill_path_variables(url.trim(), &path_values);
    if url.is_empty() {
        return Err(PrepareError::EmptyUrl);
    }
    let mut url = match url.split_once("://") {
        Some((scheme, _))
            if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") =>
        {
            url
        }
        Some((scheme, _)) if !scheme.is_empty() && scheme.chars().all(char::is_alphanumeric) => {
            return Err(PrepareError::UnsupportedScheme(scheme.to_owned()));
        }
        // `localhost:3000/x`, as browsers and other clients take it.
        _ => format!("http://{url}"),
    };

    let mut headers: Vec<(String, String)> = inherited
        .headers
        .iter()
        .chain(request.headers.active())
        .map(|kv| (render(kv.key.trim()), render(&kv.value)))
        .collect();

    let auth = if request.auth == Auth::Inherit {
        &inherited.auth
    } else {
        &request.auth
    };
    match auth.effect(&mut render) {
        Some(AuthEffect::Header(name, value)) => {
            // A header the user wrote wins.
            if !headers.iter().any(|(k, _)| k.eq_ignore_ascii_case(&name)) {
                headers.push((name, value));
            }
        }
        Some(AuthEffect::Query(key, value)) => {
            let (before, fragment) = match url.find('#') {
                Some(i) => url.split_at(i),
                None => (url.as_str(), ""),
            };
            let separator = if before.contains('?') { '&' } else { '?' };
            url = format!("{before}{separator}{key}={value}{fragment}");
        }
        None => {}
    }

    let body = prepare_body(&request.body, &mut render)?;

    Ok(PreparedRequest {
        method: request.method,
        url,
        headers,
        body,
        missing_variables: missing,
    })
}

/// The body to send, its variables resolved.
fn prepare_body(
    body: &crate::request::Body,
    render: &mut impl FnMut(&str) -> String,
) -> Result<PreparedBody, PrepareError> {
    let mut rows = |rows: &crate::key_value::KeyValues| {
        rows.active()
            .map(|kv| (render(kv.key.trim()), render(&kv.value)))
            .collect::<Vec<_>>()
    };
    Ok(match body.kind {
        BodyKind::None => PreparedBody::None,
        BodyKind::Json => PreparedBody::Json(render(&body.json)),
        BodyKind::Form => PreparedBody::Form(rows(&body.form)),
        BodyKind::Multipart => PreparedBody::Multipart(
            body.multipart
                .iter()
                .filter(|p| p.enabled && !p.key.trim().is_empty())
                .map(|p| {
                    let value = render(&p.value);
                    let value = if p.is_file {
                        PartValue::File(value)
                    } else {
                        PartValue::Text(value)
                    };
                    (render(p.key.trim()), value)
                })
                .collect(),
        ),
        BodyKind::Text => PreparedBody::Text {
            content_type: "text/plain; charset=utf-8",
            text: render(&body.text),
        },
        BodyKind::Xml => PreparedBody::Text {
            content_type: "application/xml",
            text: render(&body.xml),
        },
        BodyKind::GraphQl => {
            let query = render(&body.graphql_query);
            let variables = render(&body.graphql_variables);
            let json = if variables.trim().is_empty() {
                format!("{{\"query\":{}}}", quote(&query))
            } else {
                format!(
                    "{{\"query\":{},\"variables\":{}}}",
                    quote(&query),
                    variables.trim()
                )
            };
            PreparedBody::Json(json)
        }
        BodyKind::File => {
            let path = render(body.file.trim());
            if path.is_empty() {
                return Err(PrepareError::NoFile);
            }
            PreparedBody::File(path)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::KeyLocation;
    use crate::key_value::KeyValues;
    use crate::request::Part;

    fn vars() -> Variables {
        Variables::resolve([("host", "api.test"), ("token", "t0k"), ("id", "7")])
    }

    fn request(url: &str) -> Request {
        let mut r = Request::new("r");
        r.url = url.into();
        r.sync_params_from_url();
        r
    }

    fn run(r: &Request) -> PreparedRequest {
        prepare(r, &Inherited::default(), &vars()).unwrap()
    }

    #[test]
    fn resolves_url_headers_and_body() {
        let mut r = request("https://{{host}}/users?x={{nope}}");
        r.headers = KeyValues(vec![
            KeyValue::new("Accept", "application/json"),
            KeyValue {
                enabled: false,
                ..KeyValue::new("X-Off", "1")
            },
        ]);
        r.body.kind = BodyKind::Json;
        r.body.json = r#"{"token":"{{token}}"}"#.into();
        r.body.text = "ignored".into();
        let p = run(&r);
        assert_eq!(p.url, "https://api.test/users?x={{nope}}");
        assert_eq!(p.headers, [("Accept".into(), "application/json".into())]);
        assert_eq!(p.body, PreparedBody::Json(r#"{"token":"t0k"}"#.into()));
        assert_eq!(
            p.missing_variables.into_iter().collect::<Vec<_>>(),
            ["nope"]
        );
    }

    #[test]
    fn fills_path_variables() {
        let mut r = request("http://{{host}}/users/:id/posts/:post");
        r.path_params.0[0].value = "{{id}}".into();
        assert_eq!(run(&r).url, "http://api.test/users/7/posts/:post");
    }

    #[test]
    fn applies_inherited_headers_and_auth() {
        let mut r = request("http://h/x");
        let inherited = Inherited {
            auth: Auth::Bearer {
                token: "{{token}}".into(),
            },
            headers: vec![KeyValue::new("X-App", "su")],
        };
        let p = prepare(&r, &inherited, &vars()).unwrap();
        assert_eq!(p.header("authorization"), Some("Bearer t0k"));
        assert_eq!(p.headers[0], ("X-App".into(), "su".into()));

        r.auth = Auth::ApiKey {
            key: "key".into(),
            value: "{{token}}".into(),
            location: KeyLocation::Query,
        };
        assert_eq!(
            prepare(&r, &inherited, &vars()).unwrap().url,
            "http://h/x?key=t0k"
        );
        r.auth = Auth::None;
        assert_eq!(
            prepare(&r, &inherited, &vars())
                .unwrap()
                .header("authorization"),
            None
        );

        r.auth = Auth::Inherit;
        r.headers.0.push(KeyValue::new("Authorization", "mine"));
        assert_eq!(
            prepare(&r, &inherited, &vars())
                .unwrap()
                .header("authorization"),
            Some("mine")
        );
    }

    #[test]
    fn body_kinds() {
        let mut r = request("http://h");
        r.body.kind = BodyKind::Form;
        r.body.form = KeyValues(vec![KeyValue::new("a", "{{host}}"), KeyValue::new("", "x")]);
        assert_eq!(
            run(&r).body,
            PreparedBody::Form(vec![("a".into(), "api.test".into())])
        );

        r.body.kind = BodyKind::Multipart;
        r.body.multipart = vec![
            Part {
                key: "name".into(),
                value: "{{id}}".into(),
                is_file: false,
                enabled: true,
            },
            Part {
                key: "avatar".into(),
                value: "/tmp/a.png".into(),
                is_file: true,
                enabled: true,
            },
            Part {
                key: "off".into(),
                value: "x".into(),
                is_file: false,
                enabled: false,
            },
        ];
        assert_eq!(
            run(&r).body,
            PreparedBody::Multipart(vec![
                ("name".into(), PartValue::Text("7".into())),
                ("avatar".into(), PartValue::File("/tmp/a.png".into())),
            ])
        );

        r.body.kind = BodyKind::GraphQl;
        r.body.graphql_query = "query { user(id: \"{{id}}\") { name } }".into();
        assert_eq!(
            run(&r).body,
            PreparedBody::Json(r#"{"query":"query { user(id: \"7\") { name } }"}"#.into())
        );
        r.body.graphql_variables = r#" {"id": {{id}}} "#.into();
        assert!(
            matches!(run(&r).body, PreparedBody::Json(j) if j.ends_with(r#""variables":{"id": 7}}"#))
        );

        r.body.kind = BodyKind::Xml;
        r.body.xml = "<a/>".into();
        assert_eq!(
            run(&r).body,
            PreparedBody::Text {
                content_type: "application/xml",
                text: "<a/>".into()
            }
        );

        r.body.kind = BodyKind::File;
        assert_eq!(
            prepare(&r, &Inherited::default(), &vars()),
            Err(PrepareError::NoFile)
        );
    }

    #[test]
    fn checks_the_address() {
        let none = Inherited::default();
        assert_eq!(
            prepare(&request("  "), &none, &vars()),
            Err(PrepareError::EmptyUrl)
        );
        assert_eq!(
            prepare(&request("ftp://h"), &none, &vars()),
            Err(PrepareError::UnsupportedScheme("ftp".into()))
        );
        assert_eq!(
            run(&request("localhost:3000/x")).url,
            "http://localhost:3000/x"
        );
        assert_eq!(run(&request("HTTPS://h")).url, "HTTPS://h");
    }
}
