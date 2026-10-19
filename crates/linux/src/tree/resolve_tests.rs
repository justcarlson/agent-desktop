use agent_desktop_core::{Deadline, ErrorCode};

use super::*;
use crate::tree::resolve_fake::{FRAME_PATH, Fake, deadline, node};
use crate::tree::role_numbers::{PANEL, PUSH_BUTTON};
use crate::tree::test_support::rect;

fn files() -> Fake {
    let mut fake = Fake::new();
    fake.add(FRAME_PATH, "/toolbar", node(PANEL, "toolbar", 800, 40));
    fake.add(
        "/toolbar",
        "/opts1",
        node(PUSH_BUTTON, "View Options", 34, 34),
    );
    fake.add(
        "/toolbar",
        "/opts2",
        node(PUSH_BUTTON, "View Options", 23, 34),
    );
    fake.add(FRAME_PATH, "/back", node(PUSH_BUTTON, "Back", 30, 30));
    fake
}

fn resolved(fake: &mut Fake, entry: &agent_desktop_core::RefEntry) -> String {
    resolve(fake, entry, deadline())
        .expect("the ref resolves")
        .path()
        .to_owned()
}

fn refusal(fake: &mut Fake, entry: &agent_desktop_core::RefEntry) -> AdapterError {
    resolve(fake, entry, deadline()).expect_err("the ref is refused")
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
fn the_path_lands_on_the_second_twin_and_nothing_is_searched() {
    let mut fake = files();
    let entry = fake.entry_of("/opts2", true);
    assert_eq!(entry.scope.path.as_slice(), [0, 1]);
    assert_eq!(resolved(&mut fake, &entry), "/opts2");
    assert_eq!(fake.reads.searches, 0);
    assert_eq!(fake.reads.levels, 2);
    assert_eq!(fake.reads.evidence, 1);
}

#[test]
fn a_ref_with_no_stored_size_still_lands_positionally_on_its_twin() {
    let mut fake = files();
    let entry = fake.entry_of("/opts2", false);
    assert!(entry.geometry.bounds.is_none());
    assert_eq!(resolved(&mut fake, &entry), "/opts2");
    assert_eq!(fake.reads.searches, 0);
}

#[test]
fn a_moved_window_changes_no_answer_for_the_path_or_for_the_size_search() {
    let mut fake = files();
    let entry = fake.entry_of("/opts2", true);
    fake.shift_every_position(120, 64);
    assert_eq!(resolved(&mut fake, &entry), "/opts2");
    assert_eq!(fake.reads.searches, 0);

    let mut without_path = entry.clone();
    without_path.scope.path.clear();
    assert_eq!(resolved(&mut fake, &without_path), "/opts2");
    assert_eq!(fake.reads.searches, 1);
}

#[test]
fn twins_the_path_cannot_separate_are_ambiguous_without_a_size_that_does() {
    let mut fake = files();
    let mut entry = fake.entry_of("/opts2", false);
    entry.scope.path.clear();
    let none = refusal(&mut fake, &entry);
    assert_eq!(none.code, ErrorCode::AmbiguousTarget);
    assert_eq!(kind(&none), "resolve_ambiguous");
    assert!(!none.is_explicitly_retryable());
    assert_eq!(
        none.details.as_ref().expect("details")["candidate_count"],
        2
    );

    let mut elsewhere = fake.entry_of("/opts2", true);
    elsewhere.scope.path.clear();
    elsewhere.geometry.bounds = Some(rect(0.0, 0.0, 99.0, 34.0));
    assert_eq!(
        refusal(&mut fake, &elsewhere).code,
        ErrorCode::AmbiguousTarget
    );
}

#[test]
fn a_path_that_lands_on_something_else_is_declined_and_the_search_decides() {
    let mut fake = files();
    let mut twin = fake.entry_of("/opts2", true);
    twin.scope.path = [0, 0].into_iter().collect();
    assert_eq!(resolved(&mut fake, &twin), "/opts2");
    assert_eq!(fake.reads.searches, 1);

    let mut back = fake.entry_of("/back", true);
    back.scope.path = [0, 0].into_iter().collect();
    assert_eq!(resolved(&mut fake, &back), "/back");
}

#[test]
fn a_landing_whose_size_contradicts_the_stored_one_is_declined_even_when_its_name_matches() {
    let mut fake = files();
    let mut entry = fake.entry_of("/opts1", true);
    entry.scope.path = [0, 1].into_iter().collect();
    assert_eq!(resolved(&mut fake, &entry), "/opts1");
    assert_eq!(fake.reads.searches, 1);
}

#[test]
fn a_page_that_left_the_tree_is_stale_and_its_stored_path_is_never_read_directly() {
    let mut fake = files();
    let entry = fake.entry_of("/opts2", true);
    fake.nodes
        .get_mut(FRAME_PATH)
        .expect("frame")
        .children
        .retain(|path| path != "/toolbar");
    let error = refusal(&mut fake, &entry);
    assert_eq!(error.code, ErrorCode::StaleRef);
    assert_eq!(kind(&error), "resolve_no_candidate");
    assert!(!error.is_explicitly_retryable());
    assert_eq!(fake.reads.evidence, 0);
}

#[test]
fn a_unique_match_resolves_and_no_match_is_stale_only_when_everything_was_read() {
    let mut fake = files();
    let mut back = fake.entry_of("/back", true);
    back.scope.path.clear();
    assert_eq!(resolved(&mut fake, &back), "/back");

    fake.nodes
        .get_mut("/back")
        .expect("back")
        .read
        .node
        .properties
        .name = crate::tree::Field::Present("Close".to_owned());
    let stale = refusal(&mut fake, &back);
    assert_eq!(kind(&stale), "resolve_no_candidate");

    fake.nodes
        .get_mut("/toolbar")
        .expect("toolbar")
        .children_unread = true;
    let incomplete = refusal(&mut fake, &back);
    assert_eq!(incomplete.code, ErrorCode::AppUnresponsive);
    assert!(incomplete.is_explicitly_retryable());
}

#[test]
fn one_match_beside_an_unread_region_is_retried_and_not_resolved() {
    let mut fake = files();
    let mut back = fake.entry_of("/back", true);
    back.scope.path.clear();
    fake.nodes
        .get_mut("/toolbar")
        .expect("toolbar")
        .children_unread = true;
    let error = refusal(&mut fake, &back);
    assert_eq!(error.code, ErrorCode::AppUnresponsive);
    assert!(error.is_explicitly_retryable());
}

#[test]
fn a_childless_web_frame_is_unread_and_never_stale() {
    let source = files();
    let entry = source.entry_of("/back", true);
    let mut cold = Fake::new();
    cold.web_process = true;
    let error = refusal(&mut cold, &entry);
    assert_eq!(error.code, ErrorCode::AppUnresponsive);
    assert!(error.is_explicitly_retryable());
}

#[test]
fn a_drill_down_ref_with_a_relative_path_skips_the_path_tier() {
    let mut fake = files();
    let mut entry = fake.entry_of("/back", true);
    entry.scope.root_ref = Some("@s1:e3".to_owned());
    entry.scope.path_is_absolute = false;
    assert_eq!(resolved(&mut fake, &entry), "/back");
    assert_eq!(fake.reads.levels, 0);
    assert_eq!(fake.reads.searches, 1);
}

#[test]
fn a_deadline_that_is_already_spent_is_a_timeout_before_any_read() {
    let mut fake = files();
    let entry = fake.entry_of("/back", true);
    let spent = Deadline::after(0).expect("a zero deadline builds");
    let error = resolve(&mut fake, &entry, spent).expect_err("spent");
    assert_eq!(error.code, ErrorCode::Timeout);
    assert_eq!(fake.reads.gate, 0);
}

#[test]
fn a_nameless_ref_whose_element_changed_size_is_a_settled_miss_and_not_a_retry() {
    let mut fake = Fake::new();
    fake.add(FRAME_PATH, "/editor", node(PANEL, "", 690, 464));
    let entry = fake.entry_of("/editor", true);
    assert!(entry.identity.name.is_none());
    fake.nodes
        .get_mut("/editor")
        .expect("editor")
        .read
        .node
        .extents = crate::tree::Field::Present(crate::tree::Extents {
        x: 0,
        y: 0,
        width: 690,
        height: 463,
    });
    let error = refusal(&mut fake, &entry);
    assert_eq!(error.code, ErrorCode::StaleRef);
    assert_eq!(kind(&error), "resolve_no_candidate");
    assert!(!error.is_explicitly_retryable());
}

#[test]
fn a_twin_whose_extents_cannot_be_read_is_never_replaced_by_the_twin_of_its_size() {
    let mut fake = files();
    let mut entry = fake.entry_of("/opts2", true);
    entry.scope.path.clear();
    fake.nodes
        .get_mut("/opts1")
        .expect("opts1")
        .read
        .node
        .extents = crate::tree::Field::Present(crate::tree::Extents {
        x: 0,
        y: 0,
        width: 23,
        height: 34,
    });
    fake.nodes
        .get_mut("/opts2")
        .expect("opts2")
        .read
        .node
        .extents = crate::tree::Field::Failed("extents unavailable".to_owned());
    let error = refusal(&mut fake, &entry);
    assert_eq!(error.code, ErrorCode::AppUnresponsive);
    assert!(error.is_explicitly_retryable());
}
