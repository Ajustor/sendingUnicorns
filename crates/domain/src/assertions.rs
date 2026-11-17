//! Checks on a response, and the expressions they read (`res.status`,
//! `res.body.items[0].id`…), as in Bruno: declarative, no script engine.

use std::fmt;

use crate::json_value::{JsonValue, format_number};
use crate::response::Response;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AssertOp {
    #[default]
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Contains,
    NotContains,
    StartsWith,
    EndsWith,
    IsEmpty,
    IsNotEmpty,
    IsDefined,
    IsUndefined,
    IsNull,
    IsTrue,
    IsFalse,
    IsNumber,
    IsString,
    IsArray,
    IsObject,
}

impl AssertOp {
    pub const ALL: [Self; 21] = [
        Self::Eq,
        Self::Neq,
        Self::Gt,
        Self::Gte,
        Self::Lt,
        Self::Lte,
        Self::Contains,
        Self::NotContains,
        Self::StartsWith,
        Self::EndsWith,
        Self::IsEmpty,
        Self::IsNotEmpty,
        Self::IsDefined,
        Self::IsUndefined,
        Self::IsNull,
        Self::IsTrue,
        Self::IsFalse,
        Self::IsNumber,
        Self::IsString,
        Self::IsArray,
        Self::IsObject,
    ];

    /// Bruno's names.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Eq => "eq",
            Self::Neq => "neq",
            Self::Gt => "gt",
            Self::Gte => "gte",
            Self::Lt => "lt",
            Self::Lte => "lte",
            Self::Contains => "contains",
            Self::NotContains => "notContains",
            Self::StartsWith => "startsWith",
            Self::EndsWith => "endsWith",
            Self::IsEmpty => "isEmpty",
            Self::IsNotEmpty => "isNotEmpty",
            Self::IsDefined => "isDefined",
            Self::IsUndefined => "isUndefined",
            Self::IsNull => "isNull",
            Self::IsTrue => "isTrue",
            Self::IsFalse => "isFalse",
            Self::IsNumber => "isNumber",
            Self::IsString => "isString",
            Self::IsArray => "isArray",
            Self::IsObject => "isObject",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|op| op.as_str().eq_ignore_ascii_case(text.trim()))
    }

    /// Whether it compares with a value (`eq 200`) or stands alone (`isEmpty`).
    pub fn takes_value(self) -> bool {
        matches!(
            self,
            Self::Eq
                | Self::Neq
                | Self::Gt
                | Self::Gte
                | Self::Lt
                | Self::Lte
                | Self::Contains
                | Self::NotContains
                | Self::StartsWith
                | Self::EndsWith
        )
    }
}

impl fmt::Display for AssertOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// `res.status eq 200`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Assertion {
    pub expr: String,
    pub op: AssertOp,
    /// May hold `{{variables}}`.
    pub value: String,
    pub enabled: bool,
}

impl Assertion {
    pub fn new(expr: impl Into<String>, op: AssertOp, value: impl Into<String>) -> Self {
        Self {
            expr: expr.into(),
            op,
            value: value.into(),
            enabled: true,
        }
    }
}

impl fmt::Display for Assertion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.op.takes_value() {
            write!(f, "{} {} {}", self.expr, self.op, self.value)
        } else {
            write!(f, "{} {}", self.expr, self.op)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssertionResult {
    /// The assertion, its value resolved.
    pub label: String,
    pub passed: bool,
    /// What the expression gave (`undefined` when nothing).
    pub actual: String,
}

/// A response, ready for expressions: its body parsed once.
#[derive(Debug)]
pub struct ResponseScope<'a> {
    response: &'a Response,
    json: Option<JsonValue>,
}

impl<'a> ResponseScope<'a> {
    pub fn new(response: &'a Response) -> Self {
        let json = std::str::from_utf8(&response.body)
            .ok()
            .and_then(JsonValue::parse);
        Self { response, json }
    }

    /// The value of `res.…`; `None` when it has none (`undefined`). Anything
    /// else is a literal string.
    pub fn eval(&self, expr: &str) -> Option<JsonValue> {
        let expr = expr.trim();
        let Some(path) = expr
            .strip_prefix("res.")
            .or_else(|| (expr == "res").then_some(""))
        else {
            return Some(JsonValue::String(expr.to_owned()));
        };
        let (head, rest) = match path.find(['.', '[']) {
            Some(i) => (&path[..i], path[i..].trim_start_matches('.')),
            None => (path, ""),
        };
        let r = self.response;
        match head {
            "status" => Some(JsonValue::Number(f64::from(r.status))),
            "statusText" => Some(JsonValue::String(r.reason.clone())),
            #[allow(clippy::cast_precision_loss)] // milliseconds and bytes, far below 2^52
            "time" | "responseTime" => Some(JsonValue::Number(r.elapsed.as_millis() as f64)),
            #[allow(clippy::cast_precision_loss)]
            "size" => Some(JsonValue::Number(r.body.len() as f64)),
            "headers" if rest.is_empty() => Some(JsonValue::Object(
                r.headers
                    .iter()
                    .map(|(k, v)| (k.to_ascii_lowercase(), JsonValue::String(v.clone())))
                    .collect(),
            )),
            "headers" => {
                let name = rest
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .trim_matches(['"', '\'']);
                r.header(name).map(|v| JsonValue::String(v.to_owned()))
            }
            "body" => match &self.json {
                Some(json) => json.at(rest).cloned(),
                None if rest.is_empty() => Some(JsonValue::String(r.text())),
                None => None,
            },
            _ => None,
        }
    }

    /// Runs `assertion`, its value already resolved by `render`.
    pub fn check(
        &self,
        assertion: &Assertion,
        render: &mut impl FnMut(&str) -> String,
    ) -> AssertionResult {
        let actual = self.eval(&assertion.expr);
        let expected = render(&assertion.value);
        let passed = compare(actual.as_ref(), assertion.op, expected.trim());
        let label = Assertion {
            value: expected,
            ..assertion.clone()
        }
        .to_string();
        AssertionResult {
            label,
            passed,
            actual: actual.map_or_else(|| "undefined".to_owned(), |v| v.to_text()),
        }
    }
}

fn number(value: &JsonValue) -> Option<f64> {
    match value {
        JsonValue::Number(n) => Some(*n),
        JsonValue::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn is_empty(value: &JsonValue) -> bool {
    match value {
        JsonValue::Null => true,
        JsonValue::String(s) => s.is_empty(),
        JsonValue::Array(a) => a.is_empty(),
        JsonValue::Object(o) => o.is_empty(),
        JsonValue::Bool(_) | JsonValue::Number(_) => false,
    }
}

fn contains(value: &JsonValue, expected: &str) -> bool {
    match value {
        JsonValue::Array(items) => items.iter().any(|i| i.to_text() == expected),
        JsonValue::Object(fields) => fields.iter().any(|(k, _)| k == expected),
        other => other.to_text().contains(expected),
    }
}

#[allow(clippy::float_cmp)] // equality of parsed numbers is what the user asks for
fn compare(actual: Option<&JsonValue>, op: AssertOp, expected: &str) -> bool {
    let Some(actual) = actual else {
        return matches!(
            op,
            AssertOp::IsUndefined | AssertOp::Neq | AssertOp::NotContains
        );
    };
    let numbers = number(actual).zip(expected.parse::<f64>().ok());
    let text = actual.to_text();
    let expected_text = expected.trim_matches('"');
    match op {
        AssertOp::Eq => numbers.map_or(text == expected_text, |(a, b)| a == b),
        AssertOp::Neq => numbers.map_or(text != expected_text, |(a, b)| a != b),
        AssertOp::Gt => numbers.is_some_and(|(a, b)| a > b),
        AssertOp::Gte => numbers.is_some_and(|(a, b)| a >= b),
        AssertOp::Lt => numbers.is_some_and(|(a, b)| a < b),
        AssertOp::Lte => numbers.is_some_and(|(a, b)| a <= b),
        AssertOp::Contains => contains(actual, expected_text),
        AssertOp::NotContains => !contains(actual, expected_text),
        AssertOp::StartsWith => text.starts_with(expected_text),
        AssertOp::EndsWith => text.ends_with(expected_text),
        AssertOp::IsEmpty => is_empty(actual),
        AssertOp::IsNotEmpty => !is_empty(actual),
        AssertOp::IsDefined => true,
        AssertOp::IsUndefined => false,
        AssertOp::IsNull => *actual == JsonValue::Null,
        AssertOp::IsTrue => *actual == JsonValue::Bool(true),
        AssertOp::IsFalse => *actual == JsonValue::Bool(false),
        AssertOp::IsNumber => matches!(actual, JsonValue::Number(_)),
        AssertOp::IsString => matches!(actual, JsonValue::String(_)),
        AssertOp::IsArray => matches!(actual, JsonValue::Array(_)),
        AssertOp::IsObject => matches!(actual, JsonValue::Object(_)),
    }
}

/// What a request's checks gave once its response came.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AfterResponse {
    /// Post-response variables to set (name, value).
    pub variables: Vec<(String, String)>,
    pub results: Vec<AssertionResult>,
}

impl AfterResponse {
    pub fn passed(&self) -> usize {
        self.results.iter().filter(|r| r.passed).count()
    }

    pub fn failed(&self) -> usize {
        self.results.len() - self.passed()
    }
}

/// Runs `request`'s post-response variables and assertions on `response`;
/// `render` resolves `{{variables}}` in their values.
pub fn after_response(
    request: &crate::request::Request,
    response: &Response,
    render: &mut impl FnMut(&str) -> String,
) -> AfterResponse {
    let scope = ResponseScope::new(response);
    let variables = request
        .post_vars
        .active()
        .map(|kv| {
            let expr = render(&kv.value);
            (
                kv.key.trim().to_owned(),
                value_text(scope.eval(&expr).as_ref()),
            )
        })
        .collect();
    let results = request
        .assertions
        .iter()
        .filter(|a| a.enabled && !a.expr.trim().is_empty())
        .map(|a| scope.check(a, render))
        .collect();
    AfterResponse { variables, results }
}

/// A variable's value from an expression's result.
pub fn value_text(value: Option<&JsonValue>) -> String {
    match value {
        None => String::new(),
        Some(JsonValue::Number(n)) => format_number(*n),
        Some(v) => v.to_text(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn response() -> Response {
        Response {
            status: 201,
            reason: "Created".into(),
            headers: vec![("Content-Type".into(), "application/json".into())],
            body: br#"{"id": 7, "token": "abc", "tags": ["a","b"], "user": {"admin": false}, "none": null}"#.to_vec(),
            elapsed: Duration::from_millis(120),
        }
    }

    fn check(expr: &str, op: AssertOp, value: &str) -> bool {
        let r = response();
        ResponseScope::new(&r)
            .check(&Assertion::new(expr, op, value), &mut str::to_owned)
            .passed
    }

    #[test]
    fn evaluates_expressions() {
        let r = response();
        let scope = ResponseScope::new(&r);
        assert_eq!(scope.eval("res.status"), Some(JsonValue::Number(201.0)));
        assert_eq!(
            scope.eval("res.headers.content-type").unwrap().to_text(),
            "application/json"
        );
        assert_eq!(
            scope.eval("res.headers['Content-Type']").unwrap().to_text(),
            "application/json"
        );
        assert_eq!(scope.eval("res.body.tags[1]").unwrap().to_text(), "b");
        assert_eq!(scope.eval("res.body.nope"), None);
        assert_eq!(
            scope.eval("plain text"),
            Some(JsonValue::String("plain text".into()))
        );
        assert_eq!(value_text(scope.eval("res.body.id").as_ref()), "7");
    }

    #[test]
    fn operators() {
        use AssertOp::*;
        assert!(check("res.status", Eq, "201"));
        assert!(check("res.status", Gte, "200") && check("res.status", Lt, "300"));
        assert!(!check("res.status", Eq, "200"));
        assert!(check("res.body.token", Eq, "abc") && check("res.body.token", Eq, "\"abc\""));
        assert!(check("res.body.tags", Contains, "a") && check("res.body.tags", NotContains, "z"));
        assert!(check("res.body.user", Contains, "admin"));
        assert!(check("res.body.user.admin", IsFalse, ""));
        assert!(check("res.body.none", IsNull, "") && check("res.body.none", IsEmpty, ""));
        assert!(check("res.body.nope", IsUndefined, "") && !check("res.body.nope", IsDefined, ""));
        assert!(check("res.time", Lt, "500"));
        assert!(check("res.body.tags", IsArray, "") && check("res.body.id", IsNumber, ""));
        assert!(check("res.statusText", StartsWith, "Cre"));
    }

    #[test]
    fn after_a_response() {
        let r = response();
        let mut request = crate::request::Request::new("r");
        request
            .post_vars
            .0
            .push(crate::key_value::KeyValue::new("token", "res.body.token"));
        request.assertions = vec![
            Assertion::new("res.status", AssertOp::Eq, "201"),
            Assertion::new("res.body.id", AssertOp::Gt, "10"),
            Assertion {
                enabled: false,
                ..Assertion::new("res.status", AssertOp::Eq, "500")
            },
        ];
        let after = after_response(&request, &r, &mut str::to_owned);
        assert_eq!(after.variables, [("token".to_owned(), "abc".to_owned())]);
        assert_eq!((after.passed(), after.failed()), (1, 1));
    }

    #[test]
    fn results_show_the_resolved_value() {
        let r = response();
        let result = ResponseScope::new(&r).check(
            &Assertion::new("res.body.id", AssertOp::Eq, "{{id}}"),
            &mut |s: &str| s.replace("{{id}}", "8"),
        );
        assert_eq!(result.label, "res.body.id eq 8");
        assert_eq!(result.actual, "7");
        assert!(!result.passed);
        for op in AssertOp::ALL {
            assert_eq!(AssertOp::parse(op.as_str()), Some(op));
        }
    }
}
