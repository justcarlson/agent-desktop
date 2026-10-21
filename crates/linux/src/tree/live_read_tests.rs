use agent_desktop_core::{AdapterError, Deadline, ErrorCode, NativeHandle};

use super::*;
use crate::tree::node_evidence::evidence_for;
use crate::tree::role_numbers::PUSH_BUTTON;
use crate::tree::test_support::read_of;
use crate::tree::{AtspiElement, ancestry};

struct Counting {
    answer: Result<Liveness, AdapterError>,
    reads: usize,
    stopped: Option<String>,
    stopped_asked: usize,
}

impl Counting {
    fn alive() -> Self {
        Self {
            answer: Ok(Liveness::Alive),
            reads: 0,
            stopped: None,
            stopped_asked: 0,
        }
    }
}

impl NodeReader for Counting {
    fn read(&mut self, element: &AtspiElement) -> Result<NodeEvidence, AdapterError> {
        self.reads += 1;
        let liveness = self.answer.clone()?;
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

    fn placement(&mut self, _element: &AtspiElement, _deadline: Deadline) -> Placement {
        Placement::default()
    }

    fn global_position(&mut self, _element: &AtspiElement, _deadline: Deadline) -> bool {
        true
    }
}

fn handle() -> NativeHandle {
    let mut element = AtspiElement::new(":1.1", "/p", u32::MAX);
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
fn a_destroyed_object_is_retryable_and_an_exited_application_settles() {
    for (answer, expected, retryable) in [
        (
            Err(AdapterError::new(ErrorCode::ElementNotFound, "gone")),
            "resolve_target_destroyed",
            true,
        ),
        (
            Err(AdapterError::new(ErrorCode::AppNotFound, "left")),
            "resolve_owning_process_exited",
            false,
        ),
        (Ok(Liveness::ObjectGone), "resolve_target_destroyed", true),
        (
            Ok(Liveness::AppGone),
            "resolve_owning_process_exited",
            false,
        ),
    ] {
        let mut reader = Counting::alive();
        reader.answer = answer;
        let error = read_live(&mut reader, &handle()).expect_err("stale");
        assert_eq!(error.code, ErrorCode::StaleRef);
        assert_eq!(kind(&error), expected);
        assert_eq!(error.is_explicitly_retryable(), retryable);
        assert_eq!(reader.stopped_asked, 0);
    }
}

#[test]
fn a_node_that_did_not_answer_is_a_timeout_and_unresponsive_when_its_process_is_stopped() {
    let mut reader = Counting::alive();
    reader.answer = Ok(Liveness::TimedOut);
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
