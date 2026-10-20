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
    for (refusal, expected, retryable) in [
        (window_replaced(), "resolve_window_replaced", false),
        (app_exited(), "resolve_owning_process_exited", false),
        (object_gone(), "resolve_target_destroyed", true),
    ] {
        let mut fake = one_button();
        let entry = fake.entry_of("/back", true);
        fake.faults.gate_error = Some(refusal);
        let error = resolve(&mut fake, &entry, deadline()).expect_err("refused");
        assert_eq!(error.code, ErrorCode::StaleRef);
        assert_eq!(kind(&error), expected);
        assert_eq!(error.is_explicitly_retryable(), retryable);
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
fn a_bare_ref_is_refused_even_when_its_stored_path_still_lands() {
    let mut fake = bare_list();
    let entry = fake.entry_of("/list", false);
    assert!(entry.identity.name.is_none() && !entry.scope.path.is_empty());
    let error = resolve(&mut fake, &entry, deadline()).expect_err("no identity anchor");
    assert_eq!(error.code, ErrorCode::StaleRef);
    no_reads(&fake);
}

#[test]
fn a_bare_ref_with_no_stored_rectangle_refuses_a_live_area() {
    let mut fake = Fake::new();
    let mut list = node(LIST, "", 0, 0);
    list.node.extents = crate::tree::Field::Absent;
    fake.add(FRAME_PATH, "/list", list);
    let entry = fake.entry_of("/list", false);
    assert!(entry.geometry.bounds.is_none());
    fake.nodes.get_mut("/list").expect("list").read.node.extents =
        crate::tree::Field::Present(crate::tree::Extents {
            x: 0,
            y: 0,
            width: 700,
            height: 474,
        });
    let error = resolve(&mut fake, &entry, deadline()).expect_err("no identity anchor");
    assert_eq!(error.code, ErrorCode::StaleRef);
    no_reads(&fake);
}

#[test]
fn a_bare_ref_is_refused_when_both_extents_reads_are_unavailable() {
    let mut fake = bare_list();
    fake.nodes.get_mut("/list").expect("list").read.node.extents =
        crate::tree::Field::Failed("no geometry on this session".to_owned());
    let entry = fake.entry_of("/list", false);
    let error = resolve(&mut fake, &entry, deadline()).expect_err("no identity anchor");
    assert_eq!(error.code, ErrorCode::StaleRef);
    no_reads(&fake);
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
    let entry = sized.entry_of("/list", true);
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
fn a_bare_ref_is_refused_before_attempting_an_unreadable_path() {
    let mut fake = bare_list();
    let entry = fake.entry_of("/list", false);
    fake.faults.level_errors.insert(
        FRAME_PATH.to_owned(),
        AdapterError::new(ErrorCode::Internal, "unreadable"),
    );
    let error = resolve(&mut fake, &entry, deadline()).expect_err("no identity anchor");
    assert_eq!(error.code, ErrorCode::StaleRef);
    assert!(!error.is_explicitly_retryable());
    no_reads(&fake);
}

#[test]
fn a_bare_ref_never_retargets_a_reordered_or_replaced_same_role_sibling() {
    for replacement in [false, true] {
        let mut fake = bare_list();
        fake.add(FRAME_PATH, "/twin", node(LIST, "", 100, 0));
        let entry = fake.entry_of("/list", false);
        let children = &mut fake.nodes.get_mut(FRAME_PATH).expect("frame").children;
        if replacement {
            children[0] = "/twin".to_owned();
            children.pop();
        } else {
            children.swap(0, 2);
        }
        let error = resolve(&mut fake, &entry, deadline()).expect_err("no identity anchor");
        assert_eq!(error.code, ErrorCode::StaleRef);
        no_reads(&fake);
    }
}

#[test]
fn a_bare_ref_with_stored_zero_area_bounds_refuses_unreadable_live_extents() {
    let mut fake = bare_list();
    let entry = fake.entry_of("/list", true);
    fake.nodes.get_mut("/list").expect("list").read.node.extents =
        crate::tree::Field::Failed("extents failed".to_owned());
    let error = resolve(&mut fake, &entry, deadline()).expect_err("no identity anchor");
    assert_eq!(error.code, ErrorCode::StaleRef);
    no_reads(&fake);
}

#[test]
fn indistinguishable_sized_twins_remain_ambiguous_after_the_path_declines() {
    let mut fake = Fake::new();
    fake.add(FRAME_PATH, "/first", node(LIST, "", 100, 40));
    fake.add(FRAME_PATH, "/second", node(LIST, "", 100, 40));
    let mut entry = fake.entry_of("/first", true);
    entry.scope.path = [99].into_iter().collect();
    let error = resolve(&mut fake, &entry, deadline()).expect_err("twins are ambiguous");
    assert_eq!(error.code, ErrorCode::AmbiguousTarget);
    assert_eq!(
        error.details.as_ref().expect("details")["candidate_count"],
        2
    );
}

fn window_field(x: i32, y: i32, width: i32, height: i32) -> crate::tree::ObservedRead {
    let mut read = node(LIST, "", width, height);
    read.node.extents = crate::tree::Field::Present(crate::tree::Extents {
        x,
        y,
        width,
        height,
    });
    read.evidence_bounds = Some((
        crate::tree::Extents {
            x,
            y,
            width,
            height,
        },
        agent_desktop_core::BoundsSpace::Window,
    ));
    read
}

#[test]
fn a_window_relative_ref_resolves_its_field_after_the_siblings_swap() {
    let mut fake = Fake::new();
    fake.add(FRAME_PATH, "/first", window_field(0, 38, 420, 34));
    fake.add(FRAME_PATH, "/second", window_field(0, 80, 420, 34));
    let entry = fake.entry_of("/first", true);
    assert_eq!(
        entry.geometry.bounds_space,
        Some(agent_desktop_core::BoundsSpace::Window)
    );

    let handle = resolve(&mut fake, &entry, deadline()).expect("the field is anchored");
    assert_eq!(handle.path(), "/first");

    fake.nodes
        .get_mut(FRAME_PATH)
        .expect("frame")
        .children
        .swap(0, 1);
    let handle = resolve(&mut fake, &entry, deadline()).expect("the field moved");
    assert_eq!(handle.path(), "/first");
    assert!(
        fake.reads.searches > 0,
        "the path tier declined the swapped sibling"
    );
}

#[test]
fn a_window_relative_ref_refuses_a_replacement_sibling_of_the_same_size() {
    let mut fake = Fake::new();
    fake.add(FRAME_PATH, "/first", window_field(0, 38, 420, 34));
    fake.add(FRAME_PATH, "/second", window_field(0, 80, 420, 34));
    let mut entry = fake.entry_of("/first", true);
    entry.scope.path.clear();
    let children = &mut fake.nodes.get_mut(FRAME_PATH).expect("frame").children;
    children.remove(0);
    fake.add(FRAME_PATH, "/replacement", window_field(0, 200, 420, 34));
    let error = resolve(&mut fake, &entry, deadline()).expect_err("the replacement moved");
    assert_eq!(error.code, ErrorCode::StaleRef);
}

#[test]
fn a_retained_object_ref_follows_its_widget_after_the_siblings_swap() {
    let mut fake = Fake::new();
    fake.add(FRAME_PATH, "/first", window_field(0, 38, 420, 34));
    fake.add(FRAME_PATH, "/second", window_field(0, 80, 420, 34));
    let mut entry = fake.entry_of("/first", true);
    entry.identity.retained_object = Some("/first".to_owned());

    fake.nodes
        .get_mut(FRAME_PATH)
        .expect("frame")
        .children
        .swap(0, 1);
    let handle = resolve(&mut fake, &entry, deadline()).expect("the widget is named by its path");
    assert_eq!(handle.path(), "/first");
    assert!(fake.reads.searches > 0, "the swapped landing was declined");
}

#[test]
fn a_retained_object_ref_refuses_a_replacement_at_the_old_rectangle() {
    let mut fake = Fake::new();
    fake.add(FRAME_PATH, "/first", window_field(0, 38, 420, 34));
    fake.add(FRAME_PATH, "/second", window_field(0, 80, 420, 34));
    let mut entry = fake.entry_of("/first", true);
    entry.identity.retained_object = Some("/first".to_owned());

    let children = &mut fake.nodes.get_mut(FRAME_PATH).expect("frame").children;
    children.remove(0);
    fake.add(FRAME_PATH, "/replacement", window_field(0, 38, 420, 34));
    let error = resolve(&mut fake, &entry, deadline()).expect_err("a new object at the old place");
    assert_eq!(error.code, ErrorCode::StaleRef);
}
