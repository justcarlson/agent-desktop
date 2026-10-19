use agent_desktop_core::state;

use super::super::menu_layout::LayoutEntry;
use super::entry_state;

fn entry() -> LayoutEntry {
    LayoutEntry {
        id: 1,
        label: "Open".to_owned(),
        enabled: true,
        separator: false,
        submenu: false,
        checked: None,
        children: Vec::new(),
    }
}

#[test]
fn an_entry_under_a_shown_item_is_not_marked_offscreen() {
    let shown = entry_state(&entry(), "menuitem", false);
    assert_eq!(shown.offscreen, None);
    assert!(!shown.states.iter().any(|token| token == state::OFFSCREEN));
}

#[test]
fn an_entry_under_an_item_with_no_panel_node_is_offscreen() {
    let hidden = entry_state(&entry(), "menuitem", true);
    assert_eq!(hidden.offscreen, Some(true));
    assert!(hidden.states.iter().any(|token| token == state::OFFSCREEN));
}

#[test]
fn a_disabled_entry_keeps_its_disabled_state_when_shown() {
    let mut disabled = entry();
    disabled.enabled = false;
    let shown = entry_state(&disabled, "menuitem", false);
    assert_eq!(shown.enabled, Some(false));
    assert!(shown.states.iter().any(|token| token == state::DISABLED));
}
