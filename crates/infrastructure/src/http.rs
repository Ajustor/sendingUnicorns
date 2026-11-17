//! [`HttpClient`] over ureq: blocking, no async runtime. One agent is kept
//! while the settings stay the same, with its connections and cookie jar.

use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use application::ports::{HttpClient, HttpError};
use domain::{Cookie, HttpMethod, PartValue, PreparedBody, PreparedRequest, Response, Settings};
use percent_encoding::{AsciiSet, CONTROLS, NON_ALPHANUMERIC, utf8_percent_encode};
use ureq::tls::{RootCerts, TlsConfig};
use ureq::{Agent, Body, Proxy, RequestBuilder};

/// The most a response body may weigh before it is refused.
const BODY_LIMIT: u64 = 100 * 1024 * 1024;
const MAX_REDIRECTS: u32 = 10;

/// What a URI cannot hold as is; `%` stays, as users type `%20` themselves.
const URI_UNSAFE: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'`')
    .add(b'{')
    .add(b'}')
    .add(b'|')
    .add(b'\\')
    .add(b'^');

/// `application/x-www-form-urlencoded` keeps letters, digits and `-_.*`.
const FORM: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'*');

#[derive(Debug, Default)]
pub struct UreqHttpClient {
    /// The agent of the current settings (its cookies live in it).
    agent: Mutex<Option<(Settings, Agent)>>,
}

impl UreqHttpClient {
    fn build_agent(settings: &Settings) -> Result<Agent, HttpError> {
        let proxy = match settings.proxy.as_deref().map(str::trim) {
            Some(url) if !url.is_empty() => Some(
                Proxy::new(url).map_err(|e| HttpError::InvalidUrl(format!("proxy {url}: {e}")))?,
            ),
            _ => Proxy::try_from_env(),
        };
        Ok(Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(settings.timeout_secs.max(1))))
            // 4xx and 5xx are responses to show, not errors.
            .http_status_as_error(false)
            .max_redirects(if settings.follow_redirects {
                MAX_REDIRECTS
            } else {
                0
            })
            // Not following: the 3xx itself is the response.
            .max_redirects_will_error(settings.follow_redirects)
            .user_agent(concat!("sendingUnicorns/", env!("CARGO_PKG_VERSION")))
            .proxy(proxy)
            .tls_config(
                TlsConfig::builder()
                    .root_certs(RootCerts::PlatformVerifier)
                    .disable_verification(!settings.verify_tls)
                    .build(),
            )
            .build()
            .into())
    }

    /// The kept agent, or a new one when the settings changed. Without
    /// cookies, a fresh agent per request: nothing is kept.
    fn agent(&self, settings: &Settings) -> Result<Agent, HttpError> {
        if !settings.cookies {
            return Self::build_agent(settings);
        }
        let mut slot = self
            .agent
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((kept, agent)) = slot.as_ref()
            && kept == settings
        {
            return Ok(agent.clone());
        }
        let agent = Self::build_agent(settings)?;
        *slot = Some((settings.clone(), agent.clone()));
        Ok(agent)
    }
}

impl HttpClient for UreqHttpClient {
    fn send(&self, request: &PreparedRequest, settings: &Settings) -> Result<Response, HttpError> {
        let agent = self.agent(settings)?;
        let url = encode_uri(&request.url);
        let payload = payload(&request.body)?;
        let has_content_type = request.header("content-type").is_some();

        let started = Instant::now();
        let result = match request.method {
            HttpMethod::Get => {
                with_body(headers(agent.get(&url), request), payload, has_content_type)
            }
            HttpMethod::Delete => with_body(
                headers(agent.delete(&url), request),
                payload,
                has_content_type,
            ),
            HttpMethod::Head => with_body(
                headers(agent.head(&url), request),
                payload,
                has_content_type,
            ),
            HttpMethod::Options => with_body(
                headers(agent.options(&url), request),
                payload,
                has_content_type,
            ),
            HttpMethod::Post => send(
                headers(agent.post(&url), request),
                payload,
                has_content_type,
            ),
            HttpMethod::Put => send(headers(agent.put(&url), request), payload, has_content_type),
            HttpMethod::Patch => send(
                headers(agent.patch(&url), request),
                payload,
                has_content_type,
            ),
        };
        let mut response = result.map_err(to_http_error)?;
        let status = response.status();
        let response_headers = response
            .headers()
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().to_owned(),
                    String::from_utf8_lossy(v.as_bytes()).into_owned(),
                )
            })
            .collect();
        let body = if request.method == HttpMethod::Head {
            Vec::new()
        } else {
            response
                .body_mut()
                .with_config()
                .limit(BODY_LIMIT)
                .read_to_vec()
                .map_err(to_http_error)?
        };
        Ok(Response {
            status: status.as_u16(),
            reason: status.canonical_reason().unwrap_or_default().to_owned(),
            headers: response_headers,
            body,
            elapsed: started.elapsed(),
        })
    }

    fn cookies(&self) -> Vec<Cookie> {
        let slot = self
            .agent
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some((_, agent)) = slot.as_ref() else {
            return Vec::new();
        };
        agent
            .cookie_jar_lock()
            .iter()
            .map(|c| Cookie {
                name: c.name().to_owned(),
                value: c.value().to_owned(),
                domain: None,
                path: None,
                expires: None,
                http_only: false,
                secure: false,
            })
            .collect()
    }

    fn clear_cookies(&self) {
        let slot = self
            .agent
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((_, agent)) = slot.as_ref() {
            agent.cookie_jar_lock().clear();
        }
    }
}

/// The bytes to send and their default `Content-Type`.
struct Payload {
    content_type: String,
    bytes: Vec<u8>,
}

fn headers<B>(mut builder: RequestBuilder<B>, request: &PreparedRequest) -> RequestBuilder<B> {
    for (key, value) in &request.headers {
        builder = builder.header(key.as_str(), value.as_str());
    }
    builder
}

/// GET, DELETE… send a body only when there is one.
fn with_body(
    builder: RequestBuilder<ureq::typestate::WithoutBody>,
    payload: Option<Payload>,
    has_content_type: bool,
) -> Result<ureq::http::Response<Body>, ureq::Error> {
    match payload {
        Some(_) => send(builder.force_send_body(), payload, has_content_type),
        None => builder.call(),
    }
}

fn send(
    mut builder: RequestBuilder<ureq::typestate::WithBody>,
    payload: Option<Payload>,
    has_content_type: bool,
) -> Result<ureq::http::Response<Body>, ureq::Error> {
    match payload {
        Some(payload) => {
            if !has_content_type {
                builder = builder.header("Content-Type", payload.content_type.as_str());
            }
            builder.send(payload.bytes.as_slice())
        }
        None => builder.send_empty(),
    }
}

fn read_file(path: &str) -> Result<Vec<u8>, HttpError> {
    std::fs::read(path).map_err(|e| HttpError::Other(format!("Could not read {path}: {e}")))
}

/// `None` for no body.
fn payload(body: &PreparedBody) -> Result<Option<Payload>, HttpError> {
    let payload = |content_type: &str, bytes: Vec<u8>| {
        Some(Payload {
            content_type: content_type.to_owned(),
            bytes,
        })
    };
    Ok(match body {
        PreparedBody::None => None,
        PreparedBody::Json(text) => payload("application/json", text.clone().into_bytes()),
        PreparedBody::Text { content_type, text } => {
            payload(content_type, text.clone().into_bytes())
        }
        PreparedBody::Form(pairs) => payload(
            "application/x-www-form-urlencoded",
            encode_form(pairs).into_bytes(),
        ),
        PreparedBody::File(path) => payload(mime_of(path), read_file(path)?),
        PreparedBody::Multipart(parts) => {
            let boundary = boundary();
            payload(
                &format!("multipart/form-data; boundary={boundary}"),
                multipart(parts, &boundary)?,
            )
        }
    })
}

/// Unlikely in any field: the time in nanoseconds.
fn boundary() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    format!("----sendingUnicorns{nanos:x}")
}

/// `multipart/form-data` (RFC 7578): files are read now.
fn multipart(parts: &[(String, PartValue)], boundary: &str) -> Result<Vec<u8>, HttpError> {
    let quote = |s: &str| s.replace('"', "%22").replace(['\r', '\n'], " ");
    let mut out = Vec::new();
    for (name, value) in parts {
        out.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        match value {
            PartValue::Text(text) => {
                out.extend_from_slice(
                    format!(
                        "Content-Disposition: form-data; name=\"{}\"\r\n\r\n",
                        quote(name)
                    )
                    .as_bytes(),
                );
                out.extend_from_slice(text.as_bytes());
            }
            PartValue::File(path) => {
                let file_name = std::path::Path::new(path)
                    .file_name()
                    .map_or_else(|| path.clone(), |n| n.to_string_lossy().into_owned());
                out.extend_from_slice(
                    format!(
                        "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: {}\r\n\r\n",
                        quote(name),
                        quote(&file_name),
                        mime_of(path)
                    )
                    .as_bytes(),
                );
                out.extend_from_slice(&read_file(path)?);
            }
        }
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    Ok(out)
}

/// The usual types by extension; bytes otherwise.
fn mime_of(path: &str) -> &'static str {
    let extension = path
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "json" => "application/json",
        "xml" => "application/xml",
        "txt" | "log" => "text/plain",
        "csv" => "text/csv",
        "html" | "htm" => "text/html",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
}

/// Spaces, quotes, braces, accents… percent-encoded; the rest as typed.
pub(crate) fn encode_uri(url: &str) -> String {
    utf8_percent_encode(url, URI_UNSAFE).to_string()
}

pub(crate) fn encode_form(pairs: &[(String, String)]) -> String {
    let encode = |s: &str| utf8_percent_encode(s, FORM).to_string().replace("%20", "+");
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn to_http_error(error: ureq::Error) -> HttpError {
    match error {
        ureq::Error::Timeout(_) => HttpError::Timeout,
        ureq::Error::BadUri(e) => HttpError::InvalidUrl(e),
        ureq::Error::Http(e) => HttpError::InvalidUrl(e.to_string()),
        ureq::Error::HostNotFound => HttpError::Connect("host not found".into()),
        ureq::Error::ConnectionFailed => HttpError::Connect("connection refused".into()),
        ureq::Error::Io(e) => HttpError::Connect(e.to_string()),
        ureq::Error::Tls(e) => HttpError::Tls(e.to_owned()),
        ureq::Error::Rustls(e) => HttpError::Tls(e.to_string()),
        ureq::Error::BodyExceedsLimit(limit) => HttpError::Other(format!(
            "The response is larger than {} MB.",
            limit / 1024 / 1024
        )),
        other => HttpError::Other(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn encodes_what_uris_cannot_hold() {
        assert_eq!(
            encode_uri("http://h/a b?q=é&x=%20"),
            "http://h/a%20b?q=%C3%A9&x=%20"
        );
    }

    #[test]
    fn encodes_forms() {
        let pairs = [("a b".to_owned(), "x&y=é".to_owned())];
        assert_eq!(encode_form(&pairs), "a+b=x%26y%3D%C3%A9");
    }

    #[test]
    fn builds_multipart_bodies() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.json");
        std::fs::write(&file, "{}").unwrap();
        let parts = [
            ("name".to_owned(), PartValue::Text("Ada".into())),
            (
                "doc".to_owned(),
                PartValue::File(file.to_string_lossy().into_owned()),
            ),
        ];
        let body = String::from_utf8(multipart(&parts, "XX").unwrap()).unwrap();
        assert_eq!(
            body,
            "--XX\r\nContent-Disposition: form-data; name=\"name\"\r\n\r\nAda\r\n\
             --XX\r\nContent-Disposition: form-data; name=\"doc\"; filename=\"a.json\"\r\nContent-Type: application/json\r\n\r\n{}\r\n\
             --XX--\r\n"
        );
        let missing = [("f".to_owned(), PartValue::File("/no/such/file".into()))];
        assert!(multipart(&missing, "XX").is_err());
    }

    /// A server answering each `replies` in turn, handing back the raw
    /// requests.
    fn serve(replies: Vec<&'static str>) -> (String, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let handle = thread::spawn(move || {
            let mut seen = Vec::new();
            for reply in replies {
                let (stream, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(stream);
                let mut head = String::new();
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        length = v.trim().parse().unwrap();
                    }
                    head.push_str(&line);
                    if line == "\r\n" {
                        break;
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                reader.get_mut().write_all(reply.as_bytes()).unwrap();
                seen.push(head + &String::from_utf8(body).unwrap());
            }
            seen
        });
        (address, handle)
    }

    fn get(url: String) -> PreparedRequest {
        PreparedRequest {
            method: HttpMethod::Get,
            url,
            headers: Vec::new(),
            body: PreparedBody::None,
            missing_variables: BTreeSet::new(),
        }
    }

    #[test]
    fn sends_and_reads_back_any_status() {
        let (address, server) = serve(vec![
            "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
        ]);
        let request = PreparedRequest {
            method: HttpMethod::Post,
            url: format!("{address}/a b"),
            headers: vec![("X-Test".into(), "1".into())],
            body: PreparedBody::Json("{\"k\":1}".into()),
            missing_variables: BTreeSet::new(),
        };
        let response = UreqHttpClient::default()
            .send(&request, &Settings::default())
            .unwrap();
        assert_eq!(
            (response.status, response.reason.as_str()),
            (404, "Not Found")
        );
        assert_eq!(response.body, b"{}");
        assert_eq!(response.header("content-type"), Some("application/json"));

        let raw = server.join().unwrap().remove(0);
        assert!(raw.starts_with("POST /a%20b HTTP/1.1\r\n"), "{raw}");
        assert!(
            raw.to_ascii_lowercase()
                .contains("content-type: application/json")
        );
        assert!(raw.to_ascii_lowercase().contains("x-test: 1"));
        assert!(raw.ends_with("{\"k\":1}"));
    }

    #[test]
    fn keeps_cookies_between_requests() {
        let (address, server) = serve(vec![
            "HTTP/1.1 200 OK\r\nSet-Cookie: sid=abc; Path=/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        ]);
        let client = UreqHttpClient::default();
        let settings = Settings::default();
        client
            .send(&get(format!("{address}/login")), &settings)
            .unwrap();
        assert_eq!(client.cookies()[0].name, "sid");
        client
            .send(&get(format!("{address}/me")), &settings)
            .unwrap();
        let seen = server.join().unwrap();
        assert!(
            seen[1].to_ascii_lowercase().contains("cookie: sid=abc"),
            "{}",
            seen[1]
        );
        client.clear_cookies();
        assert!(client.cookies().is_empty());
    }

    #[test]
    fn a_closed_port_is_a_connection_error() {
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        assert!(matches!(
            UreqHttpClient::default().send(
                &get(format!("http://127.0.0.1:{port}/")),
                &Settings::default()
            ),
            Err(HttpError::Connect(_))
        ));
    }

    #[test]
    fn a_bad_proxy_is_refused() {
        let settings = Settings {
            proxy: Some("::nope".into()),
            ..Settings::default()
        };
        assert!(matches!(
            UreqHttpClient::default().send(&get("http://127.0.0.1:1/".into()), &settings),
            Err(HttpError::InvalidUrl(_))
        ));
    }
}
