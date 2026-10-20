use agent_desktop_core::ErrorCode;

use super::{choose, is_menu_entry, menu_closed, no_position, resolve_contextual};
use crate::tree::AtspiElement;
use crate::tree::menu_context::MenuContext;

fn element(path: &str) -> AtspiElement {
    AtspiElement::new(":1.2", path, 7)
}

fn kind_of(error: &agent_desktop_core::AdapterError) -> Option<&str> {
    error
        .details
        .as_ref()
        .and_then(|details| details.get("kind"))
        .and_then(serde_json::Value::as_str)
}

#[test]
fn a_ref_no_open_popup_answers_to_is_stale_because_the_menu_closed() {
    let error = choose(Vec::<AtspiElement>::new(), None).expect_err("no landing");
    assert_eq!(error.code, ErrorCode::StaleRef);
    assert_eq!(kind_of(&error), Some("menu_closed"));
    assert_eq!(menu_closed().code, ErrorCode::StaleRef);
}

#[test]
fn one_popup_answering_resolves_the_ref() {
    let landed = choose(vec![element("/item")], None).expect("a landing");
    assert_eq!(landed.path(), "/item");
}

#[test]
fn two_popups_answering_are_ambiguous() {
    let error = choose(vec![element("/a"), element("/b")], None).expect_err("two landings");
    assert_eq!(error.code, ErrorCode::AmbiguousTarget);
}

#[test]
fn a_replacement_popup_with_the_same_item_path_cannot_resolve_an_old_ref() {
    let original = MenuContext::test("/popup/a", 10);
    let replacement = MenuContext::test("/popup/b", 20);
    let mut entry = crate::tree::test_support::ref_entry("menuitem");
    entry.identity.retained_object = Some(original.encode("/shared/item"));
    let mut attempted = false;

    let result = resolve_contextual(
        &entry,
        vec![(replacement, element("/popup/b"))],
        |_root, _entry| {
            attempted = true;
            Ok(element("/shared/item"))
        },
    );

    assert_eq!(
        result
            .expect_err("the ref belongs to the closed popup")
            .code,
        agent_desktop_core::ErrorCode::StaleRef
    );
    assert!(
        !attempted,
        "the replacement popup must not reach child resolution"
    );
}

#[test]
fn a_refusal_that_is_not_staleness_is_kept_when_nothing_landed() {
    let refusal = agent_desktop_core::AdapterError::timeout("slow");
    let error = choose(Vec::<AtspiElement>::new(), Some(refusal)).expect_err("a refusal");
    assert_eq!(error.code, ErrorCode::Timeout);
}

#[test]
fn a_landing_beats_a_refusal_from_another_popup() {
    let refusal = agent_desktop_core::AdapterError::timeout("slow");
    let landed = choose(vec![element("/item")], Some(refusal)).expect("a landing");
    assert_eq!(landed.path(), "/item");
}

#[test]
fn a_ref_taken_on_the_menu_surface_is_a_menu_entry_and_a_window_ref_is_not() {
    let mut entry = crate::tree::test_support::ref_entry("menuitem");
    entry.source.source_surface = agent_desktop_core::SnapshotSurface::Menu;
    assert!(is_menu_entry(&entry));
    entry.source.source_surface = agent_desktop_core::SnapshotSurface::Window;
    assert!(!is_menu_entry(&entry));
}

#[test]
fn an_unplaced_popup_item_is_refused_at_once_before_any_event() {
    let error = no_position();
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(
        error.disposition,
        agent_desktop_core::DeliverySemantics::not_delivered()
    );
    let details = error.details.expect("details");
    assert_eq!(details["physical_delivery_started"], false);
    assert_eq!(details["check"], "geometry");
    assert_eq!(details["retryable"], false);
}
