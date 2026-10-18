use agent_desktop_core::{AdapterError, ErrorCode};

use super::*;
use crate::tree::resolve_errors::{app_exited, object_gone, window_replaced};
use crate::tree::resolve_fake::{FRAME_PATH, Fake, deadline, node};
use crate::tree::role_numbers::PUSH_BUTTON;
use crate::tree::test_support::{rect, ref_entry};

fn one_button() -> Fake {
    let mut fake = Fake::new();
    fake.add(FRAME_PATH, "/back", node(PUSH_BUTTON, "Back", 30, 30));
    fake
}

fn kind(error: &AdapterError) -> &str {
    error
        .details
        .as_ref()
        .and_then(|details| details.get("kind"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
}

fn no_reads(fake: &Fake) {
    assert_eq!(fake.reads.levels, 0);
    assert_eq!(fake.reads.evidence, 0);
    assert_eq!(fake.reads.searches, 0);
}

#[test]
fn a_ref_with_nothing_to_match_is_refused_before_any_read_of_any_kind() {
    let mut fake = one_button();
    let mut entry = ref_entry("group");
    entry.geometry.bounds = Some(rect(0.0, 0.0, 710.0, 0.0));
    let error = resolve(&mut fake, &entry, deadline()).expect_err("refused");
    assert_eq!(error.code, ErrorCode::StaleRef);
    assert_eq!(kind(&error), "resolve_unverifiable_entry");
    assert_eq!(fake.reads.gate, 0);
    no_reads(&fake);
}

#[test]
fn a_gate_that_refuses_ends_the_resolution_before_the_path_or_the_search() {
    for (refusal, expected) in [
        (window_replaced(), "resolve_window_replaced"),
        (app_exited(), "resolve_owning_process_exited"),
        (object_gone(), "resolve_target_destroyed"),
    ] {
        let mut fake = one_button();
        let entry = fake.entry_of("/back", true);
        fake.gate_error = Some(refusal);
        let error = resolve(&mut fake, &entry, deadline()).expect_err("refused");
        assert_eq!(error.code, ErrorCode::StaleRef);
        assert_eq!(kind(&error), expected);
        assert!(!error.is_explicitly_retryable());
        assert_eq!(fake.reads.gate, 1);
        no_reads(&fake);
    }
}

#[test]
fn an_application_that_left_during_the_gate_is_a_settled_stale_ref() {
    let mut fake = one_button();
    let entry = fake.entry_of("/back", true);
    fake.gate_error = Some(AdapterError::new(ErrorCode::AppNotFound, "left"));
    let error = resolve(&mut fake, &entry, deadline()).expect_err("gone");
    assert_eq!(kind(&error), "resolve_owning_process_exited");
}

#[test]
fn a_timeout_is_never_stale_and_a_stopped_process_makes_it_unresponsive() {
    let mut fake = one_button();
    let mut entry = fake.entry_of("/back", true);
    entry.scope.path.clear();
    fake.search_error = Some(AdapterError::timeout("slow"));
    let slow = resolve(&mut fake, &entry, deadline()).expect_err("timeout");
    assert_eq!(slow.code, ErrorCode::Timeout);

    fake.stopped = Some("files".to_owned());
    let stopped = resolve(&mut fake, &entry, deadline()).expect_err("stopped");
    assert_eq!(stopped.code, ErrorCode::AppUnresponsive);
}

#[test]
fn a_timeout_while_the_gate_waits_on_a_stopped_process_is_unresponsive() {
    let mut fake = one_button();
    let entry = fake.entry_of("/back", true);
    fake.gate_error = Some(AdapterError::timeout("slow"));
    fake.stopped = Some("files".to_owned());
    let error = resolve(&mut fake, &entry, deadline()).expect_err("stopped");
    assert_eq!(error.code, ErrorCode::AppUnresponsive);
    no_reads(&fake);
}
