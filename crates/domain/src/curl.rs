//! cURL commands: pasted ones become requests (as browsers' "Copy as
//! cURL" writes them), and any request can be copied as one.

use std::fmt;

use crate::auth::Auth;
use crate::json_value::JsonValue;
use crate::key_value::{KeyValue, KeyValues};
use crate::prepare::{PartValue, PreparedBody, PreparedRequest};
use crate::request::{BodyKind, HttpMethod, Part, Request};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CurlError {
    NotCurl,
    Unclosed,
    NoUrl,
    /// An option that needs a value got none.
    MissingValue(String),
}

impl fmt::Display for CurlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotCurl => f.write_str("this is not a curl command"),
            Self::Unclosed => f.write_str("a quote is not closed"),
            Self::NoUrl => f.write_str("the command has no address"),
            Self::MissingValue(o) => write!(f, "{o} needs a value"),
        }
    }
}

impl std::error::Error for CurlError {}

/// Shell words: quotes, `$'…'`, backslashes and line continuations (`\`,
/// `^` from Windows' cmd).
fn words(command: &str) -> Result<Vec<String>, CurlError> {
    let mut words = Vec::new();
    let mut chars = command.chars().peekable();
    let mut current: Option<String> = None;
    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {
                if let Some(w) = current.take() {
                    words.push(w);
                }
            }
            '\\' | '^' if matches!(chars.peek(), Some('\n' | '\r')) => {
                while matches!(chars.peek(), Some('\n' | '\r')) {
                    chars.next();
                }
            }
            '\\' => {
                if let Some(next) = chars.next() {
                    current.get_or_insert_default().push(next);
                }
            }
            '\'' => {
                let word = current.get_or_insert_default();
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(c) => word.push(c),
                        None => return Err(CurlError::Unclosed),
                    }
                }
            }
            '$' if chars.peek() == Some(&'\'') => {
                chars.next();
                let word = current.get_or_insert_default();
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some('\\') => match chars.next() {
                            Some('n') => word.push('\n'),
                            Some('t') => word.push('\t'),
                            Some('r') => word.push('\r'),
                            Some(c) => word.push(c),
                            None => return Err(CurlError::Unclosed),
                        },
                        Some(c) => word.push(c),
                        None => return Err(CurlError::Unclosed),
                    }
                }
            }
            '"' => {
                let word = current.get_or_insert_default();
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some(c @ ('"' | '\\' | '$' | '`')) => word.push(c),
                            Some('\n') => {}
                            Some(c) => {
                                word.push('\\');
                                word.push(c);
                            }
                            None => return Err(CurlError::Unclosed),
                        },
                        Some(c) => word.push(c),
                        None => return Err(CurlError::Unclosed),
                    }
                }
            }
            c => current.get_or_insert_default().push(c),
        }
    }
    if let Some(w) = current {
        words.push(w);
    }
    Ok(words)
}

/// `%41+b` → `A b`.
fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
                match hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                    Some(b) => {
                        out.push(b);
                        i += 2;
                    }
                    None => out.push(b'%'),
                }
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn form_rows(data: &str) -> KeyValues {
    data.split('&')
        .filter(|p| !p.is_empty())
        .map(|pair| match pair.split_once('=') {
            Some((k, v)) => KeyValue::new(decode(k), decode(v)),
            None => KeyValue::new(decode(pair), ""),
        })
        .collect()
}

/// What a curl command's options say.
#[derive(Debug, Default)]
struct Options {
    method: Option<HttpMethod>,
    url: Option<String>,
    headers: KeyValues,
    data: Vec<String>,
    urlencoded: KeyValues,
    parts: Vec<Part>,
    json: Option<String>,
    auth: Option<Auth>,
    /// `-G`: the data goes in the query.
    get: bool,
}

/// `--data=x`, `-XPOST`: an option and its value written with it.
fn split_inline(arg: &str) -> (String, Option<String>) {
    match arg.split_once('=') {
        Some((o, v)) if o.starts_with("--") => (o.to_owned(), Some(v.to_owned())),
        _ if arg.len() > 2
            && arg.starts_with('-')
            && !arg.starts_with("--")
            && "XHduFbAe".contains(&arg[1..2]) =>
        {
            (arg[..2].to_owned(), Some(arg[2..].to_owned()))
        }
        _ => (arg.to_owned(), None),
    }
}

fn read_options(args: impl Iterator<Item = String>) -> Result<Options, CurlError> {
    let mut o = Options::default();
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        let (option, inline) = split_inline(&arg);
        let mut value = |option: &str| {
            inline
                .clone()
                .or_else(|| args.next())
                .ok_or_else(|| CurlError::MissingValue(option.to_owned()))
        };
        match option.as_str() {
            "-X" | "--request" => o.method = HttpMethod::parse(&value(&option)?),
            "-H" | "--header" => {
                let header = value(&option)?;
                if let Some((k, v)) = header.split_once(':') {
                    o.headers.0.push(KeyValue::new(k.trim(), v.trim()));
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" | "--data-ascii" => {
                o.data.push(value(&option)?);
            }
            "--data-urlencode" => {
                let pair = value(&option)?;
                let (k, v) = pair.split_once('=').unwrap_or(("", &pair));
                o.urlencoded.0.push(KeyValue::new(k, v));
            }
            "--json" => o.json = Some(value(&option)?),
            "-F" | "--form" | "--form-string" => {
                let field = value(&option)?;
                o.parts.push(part(&field, option != "--form-string"));
            }
            "-u" | "--user" => {
                let user = value(&option)?;
                let (u, p) = user.split_once(':').unwrap_or((&user, ""));
                o.auth = Some(Auth::Basic {
                    username: u.to_owned(),
                    password: p.to_owned(),
                });
            }
            "-b" | "--cookie" => o.headers.0.push(KeyValue::new("Cookie", value(&option)?)),
            "-A" | "--user-agent" => o
                .headers
                .0
                .push(KeyValue::new("User-Agent", value(&option)?)),
            "-e" | "--referer" => o.headers.0.push(KeyValue::new("Referer", value(&option)?)),
            "--url" => o.url = Some(value(&option)?),
            "-G" | "--get" => o.get = true,
            "-I" | "--head" => o.method = Some(HttpMethod::Head),
            // Options with a value that change nothing here.
            "-o" | "--output" | "-m" | "--max-time" | "--connect-timeout" | "-x" | "--proxy"
            | "-w" | "--write-out" | "--retry" | "-c" | "--cookie-jar" | "--cacert" | "--cert"
            | "--key" => {
                value(&option)?;
            }
            other if other.starts_with('-') => {}
            _ => o.url = o.url.or(Some(arg)),
        }
    }
    Ok(o)
}

/// `-F name=value`, or `-F file=@path;type=…`.
fn part(field: &str, files: bool) -> Part {
    let (k, v) = field.split_once('=').unwrap_or((field, ""));
    let is_file = files && v.starts_with('@');
    Part {
        key: k.to_owned(),
        value: if is_file {
            v[1..].split(';').next().unwrap_or_default().to_owned()
        } else {
            v.to_owned()
        },
        is_file,
        enabled: true,
    }
}

/// A request from a curl command (named after its address).
///
/// # Errors
///
/// When the text is not a curl command with an address.
pub fn parse_curl(command: &str) -> Result<Request, CurlError> {
    let mut args = words(command.trim())?.into_iter();
    if args
        .next()
        .is_none_or(|w| !w.ends_with("curl") && !w.ends_with("curl.exe"))
    {
        return Err(CurlError::NotCurl);
    }
    let options = read_options(args)?;
    let url = options.url.clone().ok_or(CurlError::NoUrl)?;
    let mut request = Request::new(name_of(&url));
    request.url = url;
    request.auth = options.auth.clone().unwrap_or(Auth::Inherit);
    let method = options.method;
    body(&mut request, options);
    let has_body = request.body.kind != BodyKind::None;
    request.method = method.unwrap_or(if has_body {
        HttpMethod::Post
    } else {
        HttpMethod::Get
    });
    // The form sets its own Content-Type (with its boundary).
    if request.body.kind == BodyKind::Multipart {
        request
            .headers
            .0
            .retain(|kv| !kv.key.eq_ignore_ascii_case("content-type"));
    }
    request.sync_params_from_url();
    Ok(request)
}

/// The body (or, with `-G`, the query) the options describe.
fn body(request: &mut Request, options: Options) {
    request.headers = options.headers;
    let content_type = request
        .headers
        .find_ignore_case("content-type")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let joined = options.data.join("&");
    let b = &mut request.body;
    if options.get && !joined.is_empty() {
        let separator = if request.url.contains('?') { '&' } else { '?' };
        request.url = format!("{}{separator}{joined}", request.url);
    } else if let Some(json) = options.json {
        b.kind = BodyKind::Json;
        b.json = json;
        if content_type.is_empty() {
            request
                .headers
                .0
                .push(KeyValue::new("Content-Type", "application/json"));
        }
    } else if !options.parts.is_empty() {
        b.kind = BodyKind::Multipart;
        b.multipart = options.parts;
    } else if !options.urlencoded.is_empty()
        || (!joined.is_empty() && content_type.contains("x-www-form-urlencoded"))
    {
        b.kind = BodyKind::Form;
        b.form = form_rows(&joined);
        b.form.0.extend(options.urlencoded.0);
    } else if !joined.is_empty() {
        if content_type.contains("json") || JsonValue::parse(&joined).is_some() {
            b.kind = BodyKind::Json;
            b.json = joined;
        } else if content_type.contains("xml") {
            b.kind = BodyKind::Xml;
            b.xml = joined;
        } else if content_type.is_empty()
            && joined.contains('=')
            && !joined.contains(char::is_whitespace)
        {
            b.kind = BodyKind::Form;
            b.form = form_rows(&joined);
        } else {
            b.kind = BodyKind::Text;
            b.text = joined;
        }
    }
}

/// The last segment of the address's path, else its host.
fn name_of(url: &str) -> String {
    let without_query = url.split(['?', '#']).next().unwrap_or(url);
    let after_scheme = without_query
        .split_once("://")
        .map_or(without_query, |(_, r)| r);
    let mut segments = after_scheme.split('/').filter(|s| !s.is_empty());
    let host = segments.next().unwrap_or("Request").to_owned();
    segments.next_back().map_or(host, ToOwned::to_owned)
}

/// `text` between single quotes, for a POSIX shell.
fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// A curl command sending `request`.
pub fn to_curl(request: &PreparedRequest) -> String {
    let mut lines = vec![format!(
        "curl -X {} {}",
        request.method,
        shell_quote(&request.url)
    )];
    for (k, v) in &request.headers {
        lines.push(format!("-H {}", shell_quote(&format!("{k}: {v}"))));
    }
    let content_type = |lines: &mut Vec<String>, value: &str| {
        if request.header("content-type").is_none() {
            lines.push(format!(
                "-H {}",
                shell_quote(&format!("Content-Type: {value}"))
            ));
        }
    };
    match &request.body {
        PreparedBody::None => {}
        PreparedBody::Json(json) => {
            content_type(&mut lines, "application/json");
            lines.push(format!("--data-raw {}", shell_quote(json)));
        }
        PreparedBody::Text {
            content_type: ct,
            text,
        } => {
            content_type(&mut lines, ct);
            lines.push(format!("--data-raw {}", shell_quote(text)));
        }
        PreparedBody::Form(pairs) => {
            for (k, v) in pairs {
                lines.push(format!(
                    "--data-urlencode {}",
                    shell_quote(&format!("{k}={v}"))
                ));
            }
        }
        PreparedBody::Multipart(parts) => {
            for (k, v) in parts {
                let field = match v {
                    PartValue::Text(t) => format!("{k}={t}"),
                    PartValue::File(p) => format!("{k}=@{p}"),
                };
                lines.push(format!("-F {}", shell_quote(&field)));
            }
        }
        PreparedBody::File(path) => {
            content_type(&mut lines, "application/octet-stream");
            lines.push(format!(
                "--data-binary {}",
                shell_quote(&format!("@{path}"))
            ));
        }
    }
    lines.join(" \\\n  ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn parses_a_browser_copy() {
        let command = r#"curl 'https://api.test/v1/users?page=2' \
  -H 'accept: application/json' \
  -H $'x-note: it\'s' \
  --data-raw '{"name":"Ada"}' \
  --compressed"#;
        let r = parse_curl(command).unwrap();
        assert_eq!(r.method, HttpMethod::Post);
        assert_eq!(r.url, "https://api.test/v1/users?page=2");
        assert_eq!(r.name, "users");
        assert_eq!(r.params.0, [KeyValue::new("page", "2")]);
        assert_eq!(r.headers.find_ignore_case("x-note"), Some("it's"));
        assert_eq!(r.body.kind, BodyKind::Json);
        assert_eq!(r.body.json, r#"{"name":"Ada"}"#);
    }

    #[test]
    fn parses_forms_auth_and_methods() {
        let r =
            parse_curl("curl -XPUT -u ada:secret http://h/x -d a=1 -d 'b=hello+world%21'").unwrap();
        assert_eq!(r.method, HttpMethod::Put);
        assert_eq!(
            r.auth,
            Auth::Basic {
                username: "ada".into(),
                password: "secret".into()
            }
        );
        assert_eq!(r.body.kind, BodyKind::Form);
        assert_eq!(
            r.body.form.0,
            [KeyValue::new("a", "1"), KeyValue::new("b", "hello world!")]
        );

        let r = parse_curl("curl http://h/upload -F name=x -F 'file=@/tmp/a b.png;type=image/png' -H 'Content-Type: multipart/form-data'").unwrap();
        assert_eq!(r.body.kind, BodyKind::Multipart);
        assert_eq!(r.body.multipart[1].value, "/tmp/a b.png");
        assert!(r.body.multipart[1].is_file);
        assert!(r.headers.is_empty());

        let r = parse_curl("curl -G http://h/s -d q=rust").unwrap();
        assert_eq!(
            (r.method, r.url.as_str()),
            (HttpMethod::Get, "http://h/s?q=rust")
        );
        let r = parse_curl("curl.exe \"http://h/\" ^\n --json \"{\\\"a\\\":1}\"").unwrap();
        assert_eq!(
            (r.method, r.body.json.as_str()),
            (HttpMethod::Post, r#"{"a":1}"#)
        );
    }

    #[test]
    fn refuses_what_is_not_curl() {
        assert_eq!(parse_curl("wget http://h"), Err(CurlError::NotCurl));
        assert_eq!(parse_curl("curl -H 'a: b'"), Err(CurlError::NoUrl));
        assert_eq!(parse_curl("curl 'http://h"), Err(CurlError::Unclosed));
        assert_eq!(
            parse_curl("curl http://h -H"),
            Err(CurlError::MissingValue("-H".into()))
        );
    }

    #[test]
    fn writes_commands_that_parse_back() {
        let prepared = PreparedRequest {
            method: HttpMethod::Post,
            url: "http://h/it's".into(),
            headers: vec![("Accept".into(), "*/*".into())],
            body: PreparedBody::Json(r#"{"a":"b'c"}"#.into()),
            missing_variables: BTreeSet::new(),
        };
        let command = to_curl(&prepared);
        assert!(command.starts_with("curl -X POST 'http://h/it'\\''s'"));
        let back = parse_curl(&command).unwrap();
        assert_eq!(back.url, "http://h/it's");
        assert_eq!(back.body.json, r#"{"a":"b'c"}"#);
        assert_eq!(
            back.headers.find_ignore_case("content-type"),
            Some("application/json")
        );
    }
}
