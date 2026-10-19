use super::eligible;
use crate::system::display_server::{Cause, DisplayServer, Unavailable};
use crate::tree::node_read::COMPONENT_INTERFACE;
use crate::tree::role_numbers::{
    DOCUMENT_FRAME, DOCUMENT_WEB, LIST, PANEL, PUSH_BUTTON, SCROLL_PANE, TEXT, VIEWPORT,
};

const TABLE: u32 = 55;
const TREE: u32 = 65;
const TREE_TABLE: u32 = 66;

fn x11() -> DisplayServer {
    DisplayServer::X11(":0".to_owned())
}

fn component() -> Vec<String> {
    vec![COMPONENT_INTERFACE.to_owned()]
}

#[test]
fn every_role_that_moves_content_is_eligible_on_x11() {
    for role in [
        SCROLL_PANE,
        VIEWPORT,
        LIST,
        TABLE,
        TREE,
        TREE_TABLE,
        DOCUMENT_FRAME,
        DOCUMENT_WEB,
    ] {
        assert!(eligible(role, &component(), &x11()), "role {role}");
    }
}

#[test]
fn a_button_a_label_and_a_panel_are_not_eligible() {
    for role in [PUSH_BUTTON, TEXT, PANEL] {
        assert!(!eligible(role, &component(), &x11()), "role {role}");
    }
}

#[test]
fn a_scroll_pane_is_not_eligible_on_wayland() {
    assert!(!eligible(
        SCROLL_PANE,
        &component(),
        &DisplayServer::Wayland
    ));
}

#[test]
fn a_scroll_pane_is_not_eligible_without_a_display() {
    let none = DisplayServer::Unavailable(Unavailable {
        cause: Cause::NotConfigured,
        detail: String::new(),
    });
    assert!(!eligible(SCROLL_PANE, &component(), &none));
}

#[test]
fn a_node_without_bounds_to_aim_at_is_not_eligible() {
    assert!(!eligible(SCROLL_PANE, &[], &x11()));
    let other = vec!["org.a11y.atspi.Action".to_owned()];
    assert!(!eligible(SCROLL_PANE, &other, &x11()));
}
