use agent_desktop_core::LocatorField;

use super::{capabilities_from, capability_of};
use crate::tree::Field;
use crate::tree::role_numbers::{ENTRY, PUSH_BUTTON, TEXT};
use crate::tree::state_bits::EDITABLE;
use crate::tree::test_support::words;

fn names(list: &[&str]) -> Field<Vec<String>> {
    Field::Present(list.iter().map(|name| (*name).to_owned()).collect())
}

fn known(field: LocatorField<Vec<String>>) -> Vec<String> {
    match field {
        LocatorField::Known(tokens) => tokens,
        other => panic!("expected a known capability list, got {other:?}"),
    }
}

fn of_button(actions: &Field<Vec<String>>) -> LocatorField<Vec<String>> {
    capabilities_from(
        actions,
        &Field::Present(PUSH_BUTTON),
        &Field::Present(words(&[])),
    )
}

#[test]
fn every_recorded_name_maps_as_the_table_states() {
    let cases = [
        ("click", Some("Click")),
        ("press", Some("Click")),
        ("activate", Some("Click")),
        ("doDefault", None),
        ("select", Some("Select")),
        ("open", Some("Click")),
        ("jump", Some("Click")),
        ("Toggle", Some("Toggle")),
        ("check", Some("Toggle")),
        ("uncheck", Some("Toggle")),
        ("switch", Some("Toggle")),
        ("showContextMenu", Some("RightClick")),
        ("menu", Some("RightClick")),
        ("ShowMenu", Some("RightClick")),
        ("menu.popup", Some("RightClick")),
        ("SetFocus", Some("SetFocus")),
        ("clickAncestor", None),
        ("clipboard.copy", None),
        ("window.close", None),
        ("default.activate", None),
        ("link.open", None),
    ];
    for (name, expected) in cases {
        assert_eq!(capability_of(name), expected, "{name}");
    }
}

#[test]
fn names_match_without_regard_to_case() {
    assert_eq!(capability_of("Activate"), capability_of("activate"));
    assert_eq!(capability_of("PRESS"), Some("Click"));
}

#[test]
fn a_failed_action_read_is_unknown_not_empty() {
    let unknown = of_button(&Field::Failed("timed out".to_owned()));
    assert_eq!(unknown, LocatorField::Unknown);
}

#[test]
fn a_confirmed_empty_list_and_a_missing_interface_are_known_empty() {
    assert_eq!(known(of_button(&names(&[]))), Vec::<String>::new());
    assert_eq!(known(of_button(&Field::Absent)), Vec::<String>::new());
}

#[test]
fn tokens_are_deduplicated_in_order_of_first_appearance() {
    let tokens = known(of_button(&names(&[
        "menu.popup",
        "click",
        "press",
        "clipboard.copy",
    ])));
    assert_eq!(tokens, ["RightClick", "Click"]);
}

#[test]
fn set_value_follows_the_editable_state_on_a_text_role() {
    let editable = capabilities_from(
        &Field::Absent,
        &Field::Present(ENTRY),
        &Field::Present(words(&[EDITABLE])),
    );
    assert_eq!(known(editable), ["SetValue"]);
    let read_only = capabilities_from(
        &Field::Absent,
        &Field::Present(ENTRY),
        &Field::Present(words(&[])),
    );
    assert!(known(read_only).is_empty());
    let button = capabilities_from(
        &Field::Absent,
        &Field::Present(PUSH_BUTTON),
        &Field::Present(words(&[EDITABLE])),
    );
    assert!(known(button).is_empty());
}

#[test]
fn set_value_joins_the_advertised_tokens() {
    let tokens = known(capabilities_from(
        &names(&["SetFocus"]),
        &Field::Present(TEXT),
        &Field::Present(words(&[EDITABLE])),
    ));
    assert_eq!(tokens, ["SetFocus", "SetValue"]);
}

#[test]
fn a_failed_role_or_state_read_makes_the_list_unknown() {
    let failed_role = capabilities_from(
        &names(&["click"]),
        &Field::Failed("x".to_owned()),
        &Field::Present(words(&[])),
    );
    assert_eq!(failed_role, LocatorField::Unknown);
    let failed_states = capabilities_from(
        &names(&["click"]),
        &Field::Present(PUSH_BUTTON),
        &Field::Failed("x".to_owned()),
    );
    assert_eq!(failed_states, LocatorField::Unknown);
}
