use agent_desktop_core::{IdentifierKind, Rect, state};

use super::*;

#[test]
fn the_passive_state_is_the_hidden_token_and_replaces_nothing_else() {
    let states = item_states(true, vec!["showing".into()]);
    assert_eq!(states, ["showing", state::HIDDEN]);
}

#[test]
fn an_active_item_carries_no_hidden_token_even_if_the_node_had_one() {
    let states = item_states(false, vec![state::HIDDEN.into(), "showing".into()]);
    assert_eq!(states, ["showing"]);
}

#[test]
fn an_empty_name_is_absent_not_known_empty() {
    assert_eq!(name_field(""), LocatorField::Absent);
    assert_eq!(name_field("Open"), LocatorField::Known("Open".into()));
}

#[test]
fn an_observed_rectangle_is_kept_and_the_node_is_on_screen() {
    let rect = Rect {
        x: 1094.0,
        y: 0.0,
        width: 48.0,
        height: 16.0,
    };
    assert_eq!(presence(Some(rect)), (Some(rect), false));
}

#[test]
fn no_observation_and_an_empty_rectangle_both_mean_offscreen_with_no_rectangle() {
    assert_eq!(presence(None), (None, true));
    let empty = Rect {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 12.0,
    };
    assert_eq!(presence(Some(empty)), (None, true));
}

#[test]
fn the_tray_identifier_has_a_kind_core_accepts_for_a_ref() {
    let target = TrayTarget {
        owner: ":1.5".to_owned(),
        object_path: "/StatusNotifierItem".to_owned(),
        menu_path: None,
        entry_id: None,
        revision: None,
        label_path: Vec::new(),
    };
    let evidence = identifiers(&target);
    let kinds: Vec<IdentifierKind> = evidence.identifiers().iter().map(|id| id.kind).collect();
    assert!(!kinds.is_empty());
    assert!(!kinds.contains(&IdentifierKind::Unknown));
}
