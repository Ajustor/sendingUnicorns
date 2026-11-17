//! What came back.

use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    /// `OK`, `Not Found`… (empty when the server sent none).
    pub reason: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub elapsed: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusClass {
    Informational,
    Success,
    Redirection,
    ClientError,
    ServerError,
}

/// A cookie a response set (`Set-Cookie`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub expires: Option<String>,
    pub http_only: bool,
    pub secure: bool,
}

impl Cookie {
    /// `name=value; Path=/; HttpOnly`; `None` without a name.
    pub fn parse(header: &str) -> Option<Self> {
        let mut parts = header.split(';').map(str::trim);
        let (name, value) = parts.next()?.split_once('=')?;
        if name.trim().is_empty() {
            return None;
        }
        let mut cookie = Self {
            name: name.trim().to_owned(),
            value: value.trim().trim_matches('"').to_owned(),
            domain: None,
            path: None,
            expires: None,
            http_only: false,
            secure: false,
        };
        for attribute in parts {
            let (key, value) = attribute.split_once('=').unwrap_or((attribute, ""));
            match key.trim().to_ascii_lowercase().as_str() {
                "domain" => cookie.domain = Some(value.trim().to_owned()),
                "path" => cookie.path = Some(value.trim().to_owned()),
                "expires" => cookie.expires = Some(value.trim().to_owned()),
                "max-age" => cookie.expires = Some(format!("in {} s", value.trim())),
                "httponly" => cookie.http_only = true,
                "secure" => cookie.secure = true,
                _ => {}
            }
        }
        Some(cookie)
    }
}

/// How to show a body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentKind {
    Json,
    Html,
    Xml,
    Text,
    Binary,
}

impl Response {
    pub fn status_class(&self) -> StatusClass {
        match self.status {
            0..=199 => StatusClass::Informational,
            200..=299 => StatusClass::Success,
            300..=399 => StatusClass::Redirection,
            400..=499 => StatusClass::ClientError,
            _ => StatusClass::ServerError,
        }
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// From the `Content-Type`, else from the bytes.
    pub fn content_kind(&self) -> ContentKind {
        let mime = self
            .header("content-type")
            .unwrap_or_default()
            .to_ascii_lowercase();
        if mime.contains("json") {
            return ContentKind::Json;
        }
        if mime.contains("html") {
            return ContentKind::Html;
        }
        if mime.contains("xml") {
            return ContentKind::Xml;
        }
        let Ok(text) = std::str::from_utf8(&self.body) else {
            return ContentKind::Binary;
        };
        match text.trim_start().chars().next() {
            Some('{' | '[') => ContentKind::Json,
            Some('<') => ContentKind::Xml,
            _ => ContentKind::Text,
        }
    }

    /// The cookies it set.
    pub fn cookies(&self) -> Vec<Cookie> {
        self.headers
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
            .filter_map(|(_, v)| Cookie::parse(v))
            .collect()
    }

    /// The body as text; invalid UTF-8 replaced.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(content_type: Option<&str>, body: &[u8]) -> Response {
        Response {
            status: 200,
            reason: "OK".into(),
            headers: content_type
                .map(|c| vec![("Content-Type".into(), c.into())])
                .unwrap_or_default(),
            body: body.to_vec(),
            elapsed: Duration::ZERO,
        }
    }

    #[test]
    fn content_kind_from_header_then_bytes() {
        assert_eq!(
            response(Some("application/problem+json"), b"").content_kind(),
            ContentKind::Json
        );
        assert_eq!(
            response(Some("text/html; charset=utf-8"), b"").content_kind(),
            ContentKind::Html
        );
        assert_eq!(response(None, b" [1]").content_kind(), ContentKind::Json);
        assert_eq!(response(None, b"hello").content_kind(), ContentKind::Text);
        assert_eq!(
            response(None, &[0xff, 0xfe]).content_kind(),
            ContentKind::Binary
        );
    }

    #[test]
    fn reads_set_cookie() {
        let mut r = response(None, b"");
        r.headers.push((
            "Set-Cookie".into(),
            r#"sid="abc"; Path=/; HttpOnly; Secure; Max-Age=60"#.into(),
        ));
        r.headers.push(("set-cookie".into(), "=nameless".into()));
        let cookies = r.cookies();
        assert_eq!(cookies.len(), 1);
        let c = &cookies[0];
        assert_eq!(
            (c.name.as_str(), c.value.as_str(), c.path.as_deref()),
            ("sid", "abc", Some("/"))
        );
        assert!(c.http_only && c.secure);
        assert_eq!(c.expires.as_deref(), Some("in 60 s"));
    }

    #[test]
    fn status_classes() {
        let mut r = response(None, b"");
        r.status = 404;
        assert_eq!(r.status_class(), StatusClass::ClientError);
        r.status = 503;
        assert_eq!(r.status_class(), StatusClass::ServerError);
    }
}
