use agent_desktop_core::ref_identity::identity_match;
use agent_desktop_core::{AdapterError, Deadline, IdentityMatch, LocatorField, NativeHandle};

use super::*;
use crate::tree::ancestry;
use crate::tree::node_evidence::evidence_for;
use crate::tree::role_numbers::PUSH_BUTTON;
use crate::tree::test_support::read_of;
use crate::tree::{AtspiElement, menu_context::MenuContext};

struct MenuReader {
    context: Result<Option<MenuContext>, AdapterError>,
    actual_object: Option<String>,
    context_reads: usize,
}

impl NodeReader for MenuReader {
    fn read(&mut self, element: &AtspiElement) -> Result<NodeEvidence, AdapterError> {
        let read = read_of(PUSH_BUTTON, "Open", &["click"]);
        let mut evidence = evidence_for(&read, element);
        let actual = self
            .actual_object
            .clone()
            .unwrap_or_else(|| element.path().to_owned());
        evidence.identifiers = evidence.identifiers.clone().with_retained_object(actual);
        Ok(NodeEvidence {
            read,
            evidence,
            liveness: Liveness::Alive,
        })
    }

    fn stopped_app(&mut self, _pid: u32) -> Option<String> {
        None
    }

    fn placement(&mut self, _element: &AtspiElement, _deadline: Deadline) -> Placement {
        Placement::default()
    }

    fn global_position(&mut self, _element: &AtspiElement, _deadline: Deadline) -> bool {
        true
    }

    fn menu_context(
        &mut self,
        _element: &AtspiElement,
        _deadline: Deadline,
    ) -> Result<Option<MenuContext>, AdapterError> {
        self.context_reads += 1;
        self.context.clone()
    }
}

fn deadline() -> Deadline {
    Deadline::after(5_000).expect("a deadline builds")
}

fn menu_handle() -> NativeHandle {
    let mut element = AtspiElement::new(":1.1", "/menu/open", 40);
    element.set_ancestry(ancestry::of_root(&AtspiElement::new(":1.1", "/menu", 40)));
    element.set_menu_owner(&AtspiElement::new(":1.1", "/window", 40));
    element.into_native_handle()
}

fn window_handle() -> NativeHandle {
    let mut element = AtspiElement::new(":1.1", "/menu/open", 40);
    element.set_ancestry(ancestry::of_root(&AtspiElement::new(
        ":1.1",
        "/menu/open",
        40,
    )));
    element.into_native_handle()
}

fn menu_ref(context: &MenuContext) -> agent_desktop_core::RefEntry {
    let mut entry = crate::tree::test_support::ref_entry("button");
    entry.identity.retained_object = Some(context.encode("/menu/open"));
    entry
}

fn verdict(
    entry: &agent_desktop_core::RefEntry,
    live: &agent_desktop_core::LiveElement,
) -> IdentityMatch {
    assert_eq!(entry.identity.role, live.state.role);
    let value = live
        .state
        .value
        .clone()
        .map_or(LocatorField::Absent, LocatorField::Known);
    identity_match(
        entry,
        &live.identity.name,
        &value,
        &live.identity.description,
        &live.identity.identifiers,
    )
}

fn reader(context: Option<MenuContext>) -> MenuReader {
    MenuReader {
        context: Ok(context),
        actual_object: None,
        context_reads: 0,
    }
}

#[test]
fn menu_live_identity_uses_the_fresh_context_and_actual_live_object_path() {
    let captured = MenuContext::test("/menu", 42);
    let mut reader = reader(Some(captured.clone()));

    let live = element_via(&mut reader, &menu_handle(), deadline()).expect("live menu item");

    assert_eq!(verdict(&menu_ref(&captured), &live), IdentityMatch::Match);
    let expected = captured.encode("/menu/open");
    assert_eq!(
        live.identity.identifiers.retained_object(),
        Some(expected.as_str())
    );
}

#[test]
fn changed_incomplete_or_unavailable_menu_context_mismatches_the_snapshot() {
    let captured = MenuContext::test("/menu", 42);
    for current in [
        Some(MenuContext::test("/replacement", 42)),
        Some(MenuContext::test_incomplete("/menu", 42)),
        None,
    ] {
        let mut reader = reader(current);
        let live = element_via(&mut reader, &menu_handle(), deadline()).expect("live item");
        assert_eq!(verdict(&menu_ref(&captured), &live), IdentityMatch::NoMatch);
    }
}

#[test]
fn the_same_context_does_not_hide_a_changed_live_object_path() {
    let captured = MenuContext::test("/menu", 42);
    let mut reader = reader(Some(captured.clone()));
    reader.actual_object = Some("/menu/replacement".to_owned());

    let live = element_via(&mut reader, &menu_handle(), deadline()).expect("live item");

    assert_eq!(verdict(&menu_ref(&captured), &live), IdentityMatch::NoMatch);
}

#[test]
fn a_menu_context_read_error_is_preserved() {
    let mut reader = reader(None);
    reader.context = Err(AdapterError::timeout("popup scan timed out"));

    let error = element_via(&mut reader, &menu_handle(), deadline()).expect_err("timeout");

    assert_eq!(error.code, agent_desktop_core::ErrorCode::Timeout);
}

#[test]
fn an_ordinary_window_read_preserves_raw_identity_without_querying_popup_context() {
    let mut reader = reader(Some(MenuContext::test("/menu", 42)));

    let live = element_via(&mut reader, &window_handle(), deadline()).expect("live window item");

    assert_eq!(reader.context_reads, 0);
    assert_eq!(
        live.identity.identifiers.retained_object(),
        Some("/menu/open")
    );
}
