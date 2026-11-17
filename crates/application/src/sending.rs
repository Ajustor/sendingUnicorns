//! Sending a request: resolve it (variables, inherited auth and headers),
//! hand it to the HTTP client, then run its post-response variables and
//! assertions.

use std::fmt;
use std::sync::Arc;

use domain::{
    AfterResponse, Inherited, PrepareError, PreparedRequest, RequestId, Response, Settings,
    Variables, Workspace,
};

use crate::ports::{HttpClient, HttpError};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SendError {
    UnknownRequest,
    Prepare(PrepareError),
}

impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownRequest => f.write_str("This request no longer exists."),
            Self::Prepare(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for SendError {}

#[derive(Debug, Clone)]
pub struct SendRequest {
    client: Arc<dyn HttpClient>,
}

impl SendRequest {
    pub fn new(client: Arc<dyn HttpClient>) -> Self {
        Self { client }
    }

    /// The request `id` as it will be sent. Cheap: run it on the UI thread,
    /// then [`Self::execute`] elsewhere.
    ///
    /// # Errors
    ///
    /// When the request is gone or its address is not usable.
    pub fn prepare(workspace: &Workspace, id: RequestId) -> Result<PreparedRequest, SendError> {
        let found = workspace.find(id).ok_or(SendError::UnknownRequest)?;
        let inherited = found
            .collection
            .map_or_else(Inherited::default, |c| Inherited {
                auth: c.effective_auth(id),
                headers: c.inherited_headers(id),
            });
        domain::prepare(found.request, &inherited, &workspace.variables(id))
            .map_err(SendError::Prepare)
    }

    /// The variables the request `id` sees (for the editor's colours).
    pub fn variables(workspace: &Workspace, id: RequestId) -> Variables {
        workspace.variables(id)
    }

    /// Blocks until the response arrives or the timeout.
    ///
    /// # Errors
    ///
    /// When no response came back.
    pub fn execute(
        &self,
        request: &PreparedRequest,
        settings: &Settings,
    ) -> Result<Response, HttpError> {
        self.client.send(request, settings)
    }

    /// The response of request `id` came: its post-response variables are
    /// set (runtime), its assertions run.
    pub fn after(workspace: &mut Workspace, id: RequestId, response: &Response) -> AfterResponse {
        let Some(found) = workspace.find(id) else {
            return AfterResponse::default();
        };
        let variables = workspace.variables(id);
        let after = domain::after_response(found.request, response, &mut |text| {
            variables.render(text).0
        });
        for (name, value) in &after.variables {
            if !name.is_empty() {
                workspace.set_runtime(name, value.clone());
            }
        }
        after
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fakes::FakeClient;
    use domain::{Auth, Collection, CollectionName, Folder, Item, KeyValue, Request};

    fn workspace() -> (Workspace, RequestId) {
        let name = CollectionName::parse("api").unwrap();
        let mut c = Collection::new(name.clone());
        let mut r = Request::new("users");
        r.url = "{{base}}/users".into();
        let id = r.id;
        let mut folder = Folder::new("users");
        folder.auth = Auth::Bearer {
            token: "{{token}}".into(),
        };
        folder.items.push(Item::Request(r));
        c.items.push(Item::Folder(folder));
        c.variables.0.push(KeyValue::new("token", "t"));
        c.add_environment("dev").unwrap();
        c.environment_mut("dev")
            .unwrap()
            .variables
            .0
            .push(KeyValue::new("base", "http://localhost:3000"));
        let mut ws = Workspace::new(vec![c]);
        ws.set_active_environment(&name, Some("dev".into()));
        (ws, id)
    }

    #[test]
    fn prepares_with_variables_and_inherited_auth() {
        let (ws, id) = workspace();
        let prepared = SendRequest::prepare(&ws, id).unwrap();
        assert_eq!(prepared.url, "http://localhost:3000/users");
        assert_eq!(prepared.header("Authorization"), Some("Bearer t"));
        assert!(SendRequest::variables(&ws, id).contains("base"));
    }

    #[test]
    fn without_environment_placeholders_stay() {
        let (mut ws, id) = workspace();
        ws.set_active_environment(&CollectionName::parse("api").unwrap(), None);
        let prepared = SendRequest::prepare(&ws, id).unwrap();
        assert_eq!(prepared.url, "http://{{base}}/users");
        assert_eq!(prepared.missing_variables.len(), 1);
    }

    #[test]
    fn unknown_requests_are_refused() {
        let (ws, _) = workspace();
        assert_eq!(
            SendRequest::prepare(&ws, Request::new("x").id),
            Err(SendError::UnknownRequest)
        );
    }

    #[test]
    fn execute_uses_the_client_and_after_sets_variables() {
        let (mut ws, id) = workspace();
        ws.edit_request(id)
            .unwrap()
            .post_vars
            .0
            .push(KeyValue::new("code", "res.status"));
        let prepared = SendRequest::prepare(&ws, id).unwrap();
        let client = Arc::new(FakeClient::default());
        let response = SendRequest::new(client.clone())
            .execute(&prepared, &Settings::default())
            .unwrap();
        assert_eq!(client.sent(), ["http://localhost:3000/users"]);
        let after = SendRequest::after(&mut ws, id, &response);
        assert_eq!(after.variables, [("code".to_owned(), "200".to_owned())]);
        assert_eq!(ws.variables(id).get("code"), Some("200"));
    }
}
