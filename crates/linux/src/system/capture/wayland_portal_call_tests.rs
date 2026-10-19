use std::cell::RefCell;
use std::rc::Rc;

use agent_desktop_core::{AdapterError, Deadline, ErrorCode};

use super::*;

const TOKEN: &str = "adtoken1";

struct FakePending {
    answer: Rc<RefCell<Option<Result<PortalAnswer, AdapterError>>>>,
}

impl Pending for FakePending {
    fn wait(self: Box<Self>, _deadline: Deadline) -> Result<PortalAnswer, AdapterError> {
        self.answer
            .borrow_mut()
            .take()
            .expect("a response was delivered before the wait")
    }
}

struct FakeWire {
    events: RefCell<Vec<String>>,
    answer: Rc<RefCell<Option<Result<PortalAnswer, AdapterError>>>>,
    early: Option<Result<PortalAnswer, AdapterError>>,
    handle: Option<String>,
    unique: String,
}

impl FakeWire {
    fn answering(answer: Result<PortalAnswer, AdapterError>) -> Self {
        Self {
            events: RefCell::new(Vec::new()),
            answer: Rc::new(RefCell::new(None)),
            early: Some(answer),
            handle: None,
            unique: ":1.42".into(),
        }
    }
}

impl PortalWire for FakeWire {
    fn unique_name(&self) -> String {
        self.unique.clone()
    }

    fn subscribe(
        &self,
        request_path: &str,
        _deadline: Deadline,
    ) -> Result<Box<dyn Pending>, AdapterError> {
        self.events
            .borrow_mut()
            .push(format!("subscribe {request_path}"));
        Ok(Box::new(FakePending {
            answer: Rc::clone(&self.answer),
        }))
    }

    fn call_screenshot(&self, token: &str, _deadline: Deadline) -> Result<String, AdapterError> {
        self.events.borrow_mut().push(format!("call {token}"));
        *self.answer.borrow_mut() = self.early.clone();
        Ok(self
            .handle
            .clone()
            .unwrap_or_else(|| request_path(&self.unique, token)))
    }
}

fn deadline() -> Deadline {
    Deadline::after(5_000).expect("a deadline")
}

fn granted(uri: &str) -> Result<PortalAnswer, AdapterError> {
    Ok(PortalAnswer::Granted {
        uri: Some(uri.into()),
    })
}

#[test]
fn the_request_path_follows_the_portal_naming() {
    assert_eq!(
        request_path(":1.42", TOKEN),
        "/org/freedesktop/portal/desktop/request/1_42/adtoken1"
    );
}

#[test]
fn response_zero_gives_the_result_uri() {
    let wire = FakeWire::answering(granted("file:///run/user/1000/shot.png"));
    assert_eq!(
        screenshot_uri(&wire, TOKEN, deadline()).expect("a uri"),
        "file:///run/user/1000/shot.png"
    );
}

#[test]
fn a_refusal_is_permission_denied_with_a_suggestion() {
    for code in [1, 2] {
        let wire = FakeWire::answering(Ok(PortalAnswer::Refused { code }));
        let error = screenshot_uri(&wire, TOKEN, deadline()).expect_err("refused");
        assert_eq!(error.code, ErrorCode::PermDenied);
        assert!(error.suggestion.is_some());
    }
}

#[test]
fn a_grant_without_a_file_is_an_action_failure() {
    let wire = FakeWire::answering(Ok(PortalAnswer::Granted { uri: None }));
    let error = screenshot_uri(&wire, TOKEN, deadline()).expect_err("no file");
    assert_eq!(error.code, ErrorCode::ActionFailed);
}

#[test]
fn the_subscription_precedes_the_call_so_an_early_response_is_seen() {
    let wire = FakeWire::answering(granted("file:///tmp/early.png"));
    screenshot_uri(&wire, TOKEN, deadline()).expect("the early response is kept");
    let events = wire.events.borrow();
    assert_eq!(events.len(), 2);
    assert!(events[0].starts_with("subscribe "), "{events:?}");
    assert!(events[1].starts_with("call "), "{events:?}");
}

#[test]
fn a_handle_that_is_not_the_subscribed_path_is_refused() {
    let mut wire = FakeWire::answering(granted("file:///tmp/x.png"));
    wire.handle = Some("/org/freedesktop/portal/desktop/request/1_42/other".into());
    let error = screenshot_uri(&wire, TOKEN, deadline()).expect_err("a mismatch");
    assert_eq!(error.code, ErrorCode::ActionFailed);
}

#[test]
fn a_timeout_names_the_dialog_a_person_must_approve() {
    let wire = FakeWire::answering(Err(AdapterError::timeout("late")));
    let error = screenshot_uri(&wire, TOKEN, deadline()).expect_err("a timeout");
    assert_eq!(error.code, ErrorCode::Timeout);
    assert!(
        error
            .suggestion
            .as_deref()
            .is_some_and(|text| text.contains("dialog"))
    );
}
