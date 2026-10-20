use agent_desktop_core::{AdapterError, ErrorCode};

use super::*;
use crate::tree::resolve_errors::{app_exited, object_gone, window_replaced};
use crate::tree::resolve_fake::{FRAME_PATH, Fake, deadline, node};
use crate::tree::role_numbers::{LIST, PUSH_BUTTON};
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
        fake.faults.gate_error = Some(refusal);
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
    fake.faults.gate_error = Some(AdapterError::new(ErrorCode::AppNotFound, "left"));
    let error = resolve(&mut fake, &entry, deadline()).expect_err("gone");
    assert_eq!(kind(&error), "resolve_owning_process_exited");
}

#[test]
fn a_timeout_is_never_stale_and_a_stopped_process_makes_it_unresponsive() {
    let mut fake = one_button();
    let mut entry = fake.entry_of("/back", true);
    entry.scope.path.clear();
    fake.faults.search_error = Some(AdapterError::timeout("slow"));
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
    fake.faults.gate_error = Some(AdapterError::timeout("slow"));
    fake.stopped = Some("files".to_owned());
    let error = resolve(&mut fake, &entry, deadline()).expect_err("stopped");
    assert_eq!(error.code, ErrorCode::AppUnresponsive);
    no_reads(&fake);
}

#[test]
fn a_stopped_process_is_unresponsive_before_anything_is_read_or_waited_for() {
    let mut fake = one_button();
    let entry = fake.entry_of("/back", true);
    fake.stopped = Some("files".to_owned());
    let error = resolve(&mut fake, &entry, deadline()).expect_err("stopped");
    assert_eq!(error.code, ErrorCode::AppUnresponsive);
    no_reads(&fake);
}

fn bare_list() -> Fake {
    let mut fake = Fake::new();
    fake.add(FRAME_PATH, "/list", node(LIST, "", 100, 0));
    fake.add(FRAME_PATH, "/back", node(PUSH_BUTTON, "Back", 30, 30));
    fake
}

#[test]
fn a_bare_ref_resolves_at_its_stored_path_when_the_node_there_is_still_bare() {
    let mut fake = bare_list();
    let entry = fake.entry_of("/list", false);
    assert!(entry.identity.name.is_none() && !entry.scope.path.is_empty());
    let handle = resolve(&mut fake, &entry, deadline()).expect("the bare ref lands");
    assert_eq!(handle.path(), "/list");
    assert_eq!(fake.reads.searches, 0);
}

#[test]
fn a_bare_ref_is_refused_when_the_path_lands_on_a_node_that_is_no_longer_bare() {
    let mut named = bare_list();
    let entry = named.entry_of("/list", false);
    named
        .nodes
        .get_mut("/list")
        .expect("list")
        .read
        .node
        .properties
        .name = crate::tree::Field::Present("Results".to_owned());
    let error = resolve(&mut named, &entry, deadline()).expect_err("refused");
    assert_eq!(kind(&error), "resolve_unverifiable_entry");

    let mut sized = bare_list();
    let entry = sized.entry_of("/list", false);
    sized
        .nodes
        .get_mut("/list")
        .expect("list")
        .read
        .node
        .extents = crate::tree::Field::Present(crate::tree::Extents {
        x: 0,
        y: 0,
        width: 100,
        height: 40,
    });
    let error = resolve(&mut sized, &entry, deadline()).expect_err("refused");
    assert_eq!(kind(&error), "resolve_unverifiable_entry");
    assert_eq!(sized.reads.searches, 0);
}

#[test]
fn a_bare_ref_is_refused_for_a_different_role_or_a_path_off_the_tree_and_is_never_searched() {
    let mut other_role = bare_list();
    let mut entry = other_role.entry_of("/list", false);
    entry.identity.role = "group".to_owned();
    let error = resolve(&mut other_role, &entry, deadline()).expect_err("refused");
    assert_eq!(kind(&error), "resolve_unverifiable_entry");

    let mut off_tree = bare_list();
    let mut entry = off_tree.entry_of("/list", false);
    entry.scope.path = [7, 7].into_iter().collect();
    let error = resolve(&mut off_tree, &entry, deadline()).expect_err("refused");
    assert_eq!(kind(&error), "resolve_unverifiable_entry");
    assert_eq!(off_tree.reads.searches, 0);
}

#[test]
fn a_bare_ref_whose_path_could_not_be_read_is_retried_and_not_refused_for_good() {
    let mut fake = bare_list();
    let entry = fake.entry_of("/list", false);
    fake.faults.level_errors.insert(
        FRAME_PATH.to_owned(),
        AdapterError::new(ErrorCode::Internal, "unreadable"),
    );
    let error = resolve(&mut fake, &entry, deadline()).expect_err("retried");
    assert_eq!(error.code, ErrorCode::AppUnresponsive);
    assert!(error.is_explicitly_retryable());
}
