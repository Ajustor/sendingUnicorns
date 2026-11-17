//! Domain layer: the HTTP client's business rules.
//!
//! Pure Rust, no I/O, no UI, no HTTP library. Every other layer depends on
//! this one; this one depends on nothing.

mod assertions;
mod auth;
mod collection;
mod curl;
mod environment;
mod history;
mod json;
mod json_value;
mod key_value;
mod prepare;
mod query;
mod request;
mod response;
mod settings;
mod template;
mod workspace;

pub use assertions::{
    AfterResponse, AssertOp, Assertion, AssertionResult, ResponseScope, after_response,
};
pub use auth::{Auth, AuthEffect, KeyLocation, base64};
pub use collection::{Collection, CollectionName, Folder, Item, NameError};
pub use curl::{CurlError, parse_curl, to_curl};
pub use environment::Environment;
pub use history::{History, HistoryEntry};
pub use json::{JsonToken, pretty_json, tokenize_json};
pub use json_value::{JsonValue, quote};
pub use key_value::{KeyValue, KeyValues};
pub use prepare::{Inherited, PartValue, PrepareError, PreparedBody, PreparedRequest, prepare};
pub use query::{join_query, split_query};
pub use request::{
    Body, BodyKind, HttpMethod, Part, Request, RequestId, fill_path_variables, path_variable_names,
};
pub use response::{ContentKind, Cookie, Response, StatusClass};
pub use settings::Settings;
pub use template::{Segment, Variables, segments};
pub use workspace::{Place, RequestRef, Workspace};
