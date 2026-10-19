use agent_desktop_core::{AdapterError, ErrorCode, NativeHandle};

use super::*;
use crate::tree::node_evidence::evidence_for;
use crate::tree::role_numbers::PUSH_BUTTON;
use crate::tree::test_support::read_of;
use crate::tree::{AtspiElement, ancestry};

struct Counting {
    answer: Result<(Liveness, ()), AdapterError>,
    reads: usize,
    stopped: Option<String>,
    stopped_asked: usize,
}

impl Counting {
    fn alive() -> Self {
        Self {
            answer: Ok((Liveness::Alive, ())),
            reads: 0,
            stopped: None,
            stopped_asked: 0,
        }
    }
}

impl NodeReader for Counting {
    fn read(&mut self, element: &AtspiElement) -> Result<NodeEvidence, AdapterError> {
        self.reads += 1;
        let (liveness, ()) = self.answer.clone()?;
        let read = read_of(PUSH_BUTTON, "Save", &["click"]);
        Ok(NodeEvidence {
            evidence: evidence_for(&read, element),
            read,
            liveness,
        })
    }

    fn stopped_app(&mut self, _pid: u32) -> Option<String> {
        self.stopped_asked += 1;
        self.stopped.clone()
    }
}

fn handle() -> NativeHandle {
    let mut element = AtspiElement::new(":1.1", "/p", 40);
    element.set_ancestry(ancestry::of_root(&element.clone()));
    element.into_native_handle()
}

fn kind(error: &AdapterError) -> &str {
    error
        .details
        .as_ref()
        .and_then(|details| details.get("kind"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
}

#[test]
fn every_projection_of_a_node_costs_exactly_one_node_read() {
    let mut reader = Counting::alive();
    let handle = handle();
    value_via(&mut reader, &handle).expect("value");
    assert_eq!(reader.reads, 1);
    state_via(&mut reader, &handle).expect("state");
    assert_eq!(reader.reads, 2);
    actions_via(&mut reader, &handle).expect("actions");
    assert_eq!(reader.reads, 3);
    element_via(&mut reader, &handle).expect("element");
    assert_eq!(reader.reads, 4);
    bounds_via(&mut reader, &handle).expect("bounds");
    assert_eq!(reader.reads, 5);
}

#[test]
fn each_projection_answers_with_its_own_part_of_the_one_read() {
    let mut reader = Counting::alive();
    let handle = handle();
    let element = element_via(&mut reader, &handle).expect("element");
    let actions = actions_via(&mut reader, &handle)
        .expect("actions")
        .expect("listed");
    assert_eq!(actions, element.available_actions);
    let state = state_via(&mut reader, &handle)
        .expect("state")
        .expect("read");
    assert_eq!(state.role, element.state.role);
}

#[test]
fn a_destroyed_object_and_an_exited_application_are_settled_stale_refs() {
    for (answer, expected) in [
        (
            Err(AdapterError::new(ErrorCode::ElementNotFound, "gone")),
            "resolve_target_destroyed",
        ),
        (
            Err(AdapterError::new(ErrorCode::AppNotFound, "left")),
            "resolve_owning_process_exited",
        ),
        (Ok((Liveness::ObjectGone, ())), "resolve_target_destroyed"),
        (Ok((Liveness::AppGone, ())), "resolve_owning_process_exited"),
    ] {
        let mut reader = Counting::alive();
        reader.answer = answer;
        let error = read_live(&mut reader, &handle()).expect_err("stale");
        assert_eq!(error.code, ErrorCode::StaleRef);
        assert_eq!(kind(&error), expected);
        assert!(!error.is_explicitly_retryable());
        assert_eq!(reader.stopped_asked, 0);
    }
}

#[test]
fn a_node_that_did_not_answer_is_a_timeout_and_unresponsive_when_its_process_is_stopped() {
    let mut reader = Counting::alive();
    reader.answer = Ok((Liveness::TimedOut, ()));
    let error = read_live(&mut reader, &handle()).expect_err("timeout");
    assert_eq!(error.code, ErrorCode::Timeout);

    reader.stopped = Some("Calculator".to_owned());
    let error = read_live(&mut reader, &handle()).expect_err("stopped");
    assert_eq!(error.code, ErrorCode::AppUnresponsive);

    let mut slow = Counting::alive();
    slow.answer = Err(AdapterError::timeout("deadline"));
    assert_eq!(
        read_live(&mut slow, &handle()).expect_err("timeout").code,
        ErrorCode::Timeout
    );
}

#[test]
fn a_handle_from_another_platform_or_an_empty_one_is_invalid_args() {
    let mut reader = Counting::alive();
    for foreign in [NativeHandle::new(5u32), NativeHandle::null()] {
        let error = read_live(&mut reader, &foreign).expect_err("invalid");
        assert_eq!(error.code, ErrorCode::InvalidArgs);
        assert_eq!(kind(&error), "invalid_native_handle");
    }
    assert_eq!(reader.reads, 0);
}
