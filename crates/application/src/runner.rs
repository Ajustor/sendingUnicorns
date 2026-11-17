//! The collection runner: sends a collection's (or a folder's) requests in
//! order, each one seeing the variables the previous ones set, and reports
//! their assertions.

use domain::{AfterResponse, CollectionName, HttpMethod, RequestId, Settings, Workspace};

use crate::sending::SendRequest;

/// One request of a run, once sent.
#[derive(Debug, Clone, PartialEq)]
pub struct RunStep {
    pub id: RequestId,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    /// The status, or why nothing came back.
    pub outcome: Result<u16, String>,
    pub elapsed_ms: u64,
    pub after: AfterResponse,
}

impl RunStep {
    /// Answered, and every assertion passed.
    pub fn passed(&self) -> bool {
        self.outcome.is_ok() && self.after.failed() == 0
    }
}

/// What a run tells while it goes.
#[derive(Debug, Clone, PartialEq)]
pub enum RunEvent {
    /// How many requests will be sent.
    Started(usize),
    Sending(usize, String),
    Step(RunStep),
    /// The runtime variables the run set, to keep.
    Finished(Vec<(String, String)>),
}

#[derive(Debug, Clone)]
pub struct RunCollection {
    send: SendRequest,
}

impl RunCollection {
    pub fn new(send: SendRequest) -> Self {
        Self { send }
    }

    /// Blocks until every request was sent (run it off the UI thread).
    /// `workspace` is the run's own copy; `stop` is asked before each
    /// request.
    pub fn execute(
        &self,
        mut workspace: Workspace,
        collection: &CollectionName,
        folder: Option<RequestId>,
        settings: &Settings,
        stop: &dyn Fn() -> bool,
        on_event: &mut dyn FnMut(RunEvent),
    ) {
        let ids: Vec<RequestId> = workspace
            .collection(collection)
            .map(|c| c.requests_in(folder).iter().map(|r| r.id).collect())
            .unwrap_or_default();
        on_event(RunEvent::Started(ids.len()));
        for (index, id) in ids.into_iter().enumerate() {
            if stop() {
                break;
            }
            let Some(found) = workspace.find(id) else {
                continue;
            };
            let (name, method) = (found.request.name.clone(), found.request.method);
            on_event(RunEvent::Sending(index, name.clone()));
            let sent = match SendRequest::prepare(&workspace, id) {
                Err(e) => RunStep {
                    id,
                    name,
                    method,
                    url: found.request.url.clone(),
                    outcome: Err(e.to_string()),
                    elapsed_ms: 0,
                    after: AfterResponse::default(),
                },
                Ok(prepared) => match self.send.execute(&prepared, settings) {
                    Err(e) => RunStep {
                        id,
                        name,
                        method,
                        url: prepared.url,
                        outcome: Err(e.to_string()),
                        elapsed_ms: 0,
                        after: AfterResponse::default(),
                    },
                    Ok(response) => {
                        let after = SendRequest::after(&mut workspace, id, &response);
                        RunStep {
                            id,
                            name,
                            method,
                            url: prepared.url,
                            outcome: Ok(response.status),
                            elapsed_ms: u64::try_from(response.elapsed.as_millis())
                                .unwrap_or(u64::MAX),
                            after,
                        }
                    }
                },
            };
            on_event(RunEvent::Step(sent));
        }
        on_event(RunEvent::Finished(workspace.runtime().to_vec()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fakes::FakeClient;
    use domain::{AssertOp, Assertion, Collection, Item, KeyValue, Request};
    use std::sync::Arc;

    #[test]
    fn runs_in_order_and_chains_variables() {
        let name = CollectionName::parse("api").unwrap();
        let mut c = Collection::new(name.clone());
        let mut login = Request::new("login");
        login.url = "http://h/login".into();
        login
            .post_vars
            .0
            .push(KeyValue::new("status", "res.status"));
        login.assertions = vec![Assertion::new("res.status", AssertOp::Eq, "200")];
        let mut me = Request::new("me");
        me.url = "http://h/me/{{status}}".into();
        me.assertions = vec![Assertion::new("res.status", AssertOp::Eq, "201")];
        let mut broken = Request::new("broken");
        broken.url = "ftp://h".into();
        c.items = vec![
            Item::Request(login),
            Item::Request(me),
            Item::Request(broken),
        ];
        let ws = Workspace::new(vec![c]);

        let client = Arc::new(FakeClient::default());
        let mut events = Vec::new();
        RunCollection::new(SendRequest::new(client.clone())).execute(
            ws,
            &name,
            None,
            &Settings::default(),
            &|| false,
            &mut |e| events.push(e),
        );
        assert_eq!(client.sent(), ["http://h/login", "http://h/me/200"]);
        assert_eq!(events[0], RunEvent::Started(3));
        let steps: Vec<&RunStep> = events
            .iter()
            .filter_map(|e| match e {
                RunEvent::Step(s) => Some(s),
                _ => None,
            })
            .collect();
        assert!(steps[0].passed());
        assert!(!steps[1].passed());
        assert!(steps[2].outcome.is_err());
        assert_eq!(
            events.last(),
            Some(&RunEvent::Finished(vec![("status".into(), "200".into())]))
        );
    }

    #[test]
    fn stops_when_asked() {
        let name = CollectionName::parse("api").unwrap();
        let mut c = Collection::new(name.clone());
        c.items = vec![Item::Request(Request::new("a"))];
        let mut events = Vec::new();
        RunCollection::new(SendRequest::new(Arc::new(FakeClient::default()))).execute(
            Workspace::new(vec![c]),
            &name,
            None,
            &Settings::default(),
            &|| true,
            &mut |e| events.push(e),
        );
        assert_eq!(events, [RunEvent::Started(1), RunEvent::Finished(vec![])]);
    }
}
