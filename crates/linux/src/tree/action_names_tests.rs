use agent_desktop_core::{Action, LocatorField};

use super::{capabilities, capability_of, toggle_verbs, verbs_for};
use crate::tree::Field;
use crate::tree::RawNode;
use crate::tree::role_numbers::PUSH_BUTTON;
use crate::tree::test_support::read_of;

fn node(actions: &[&str]) -> RawNode {
    read_of(PUSH_BUTTON, "n", actions).node
}

fn known(node: &RawNode) -> Vec<String> {
    match capabilities(node) {
        LocatorField::Known(tokens) => tokens,
        other => panic!("expected a known capability list, got {other:?}"),
    }
}

#[test]
fn every_recorded_name_maps_as_the_table_states() {
    let cases: [(&str, &[&str]); 22] = [
        ("click", &["Click"]),
        ("press", &["Click"]),
        ("activate", &["Click"]),
        ("doDefault", &[]),
        ("select", &["Select"]),
        ("open", &["Click"]),
        ("jump", &["Click"]),
        ("Toggle", &["Toggle"]),
        ("check", &["Toggle", "Click"]),
        ("uncheck", &["Toggle"]),
        ("switch", &["Toggle"]),
        ("showContextMenu", &["RightClick"]),
        ("menu", &["RightClick"]),
        ("ShowMenu", &["RightClick"]),
        ("menu.popup", &["RightClick"]),
        ("SetFocus", &["SetFocus"]),
        ("clickAncestor", &["Click"]),
        ("clipboard.copy", &[]),
        ("window.close", &[]),
        ("default.activate", &[]),
        ("link.open", &[]),
        ("listitem.toggle-expand", &[]),
    ];
    for (name, expected) in cases {
        assert_eq!(capability_of(name), expected, "{name}");
    }
}

#[test]
fn names_match_without_regard_to_case() {
    assert_eq!(capability_of("Activate"), capability_of("activate"));
    assert_eq!(capability_of("PRESS"), ["Click"]);
    assert_eq!(capability_of("CLICKANCESTOR"), ["Click"]);
}

#[test]
fn a_chromium_div_with_only_an_ancestor_handler_is_clickable() {
    let tokens = known(&node(&["clickAncestor", "showContextMenu"]));
    assert_eq!(tokens, ["Click", "RightClick"]);
}

#[test]
fn a_node_whose_only_name_is_do_default_has_no_token() {
    assert!(known(&node(&["doDefault"])).is_empty());
}

#[test]
fn check_gives_toggle_and_click_and_uncheck_gives_toggle_only() {
    assert_eq!(known(&node(&["check"])), ["Toggle", "Click"]);
    assert_eq!(known(&node(&["uncheck"])), ["Toggle"]);
}

#[test]
fn a_failed_action_read_is_unknown_not_empty() {
    let mut failed = node(&[]);
    failed.actions = Field::Failed("timed out".to_owned());
    assert_eq!(capabilities(&failed), LocatorField::Unknown);
}

#[test]
fn a_confirmed_empty_list_and_a_missing_interface_are_known_empty() {
    assert!(known(&node(&[])).is_empty());
    let mut absent = node(&[]);
    absent.actions = Field::Absent;
    assert!(known(&absent).is_empty());
}

#[test]
fn tokens_are_deduplicated_in_order_of_first_appearance() {
    let tokens = known(&node(&["menu.popup", "click", "press", "clipboard.copy"]));
    assert_eq!(tokens, ["RightClick", "Click"]);
}

#[test]
fn a_failed_role_or_state_read_makes_the_list_unknown() {
    let mut failed_role = node(&["click"]);
    failed_role.traits.role = Field::Failed("x".to_owned());
    assert_eq!(capabilities(&failed_role), LocatorField::Unknown);
    let mut failed_states = node(&["click"]);
    failed_states.traits.states = Field::Failed("x".to_owned());
    assert_eq!(capabilities(&failed_states), LocatorField::Unknown);
}

#[test]
fn the_check_verbs_put_check_before_the_toggle_verbs() {
    assert_eq!(
        verbs_for(&Action::Check),
        ["check", "toggle", "switch", "click"]
    );
    assert_eq!(
        verbs_for(&Action::Uncheck),
        ["uncheck", "toggle", "switch", "click", "check"]
    );
    assert_eq!(
        verbs_for(&Action::Toggle),
        ["toggle", "switch", "click", "check", "uncheck"]
    );
    assert_eq!(toggle_verbs(Some(true)), verbs_for(&Action::Check));
    assert_eq!(toggle_verbs(None), verbs_for(&Action::Toggle));
}

#[test]
fn the_click_verbs_end_with_the_two_chromium_verbs_and_exclude_do_default() {
    let verbs = verbs_for(&Action::Click);
    assert_eq!(
        verbs,
        [
            "click",
            "press",
            "activate",
            "open",
            "jump",
            "check",
            "clickAncestor"
        ]
    );
    assert!(
        !verbs
            .iter()
            .any(|verb| verb.eq_ignore_ascii_case("doDefault"))
    );
}

#[test]
fn the_disclosure_verbs_try_the_list_item_verbs_first() {
    assert_eq!(
        verbs_for(&Action::Expand),
        [
            "listitem.toggle-expand",
            "listitem.expand",
            "click",
            "press",
            "activate"
        ]
    );
    assert_eq!(verbs_for(&Action::Collapse)[1], "listitem.collapse");
}

#[test]
fn an_action_with_no_semantic_verb_has_an_empty_list() {
    assert!(verbs_for(&Action::Hover).is_empty());
    assert!(verbs_for(&Action::Clear).is_empty());
}

#[test]
fn every_verb_a_dispatch_tries_is_a_verb_the_table_reads_back() {
    let table_verbs = [
        (Action::Click, "Click"),
        (Action::RightClick, "RightClick"),
        (Action::SetFocus, "SetFocus"),
        (Action::Select(String::new()), "Select"),
    ];
    for (action, token) in table_verbs {
        for verb in verbs_for(&action) {
            assert!(
                capability_of(verb).contains(&token),
                "{verb} for {}",
                action.name()
            );
        }
    }
    for action in [Action::Toggle, Action::Check, Action::Uncheck] {
        for verb in verbs_for(&action) {
            assert!(!capability_of(verb).is_empty(), "{verb}");
        }
    }
    for action in [Action::Expand, Action::Collapse] {
        for verb in verbs_for(&action) {
            let disclosure = verb.starts_with("listitem.");
            assert!(
                disclosure || capability_of(verb).contains(&"Click"),
                "{verb}"
            );
        }
    }
}
