use super::menu_popup::Node;
use crate::tree::test_support::{read_of, words};
use crate::tree::{Field, WalkedNode};

pub(super) const SHOWING: u32 = 25;
pub(super) const HAS_POPUP: u32 = 42;

pub(super) const FRAME: u32 = 23;
pub(super) const WINDOW: u32 = 69;
pub(super) const PANEL: u32 = 39;
pub(super) const MENU: u32 = 33;
pub(super) const MENU_BAR: u32 = 34;
pub(super) const ITEM: u32 = 35;
pub(super) const POPUP_MENU: u32 = 41;
pub(super) const SEPARATOR: u32 = 50;
pub(super) const CHECK_ITEM: u32 = 8;
pub(super) const BUTTON: u32 = 43;
pub(super) const TOGGLE: u32 = 62;
pub(super) const CHECKED: u32 = 4;
pub(super) const MODAL: u32 = 16;
pub(super) const LABEL: u32 = 29;
pub(super) const DOCUMENT_WEB: u32 = 95;

pub(super) fn node(role: u32, name: &str, states: &[u32], children: Vec<Node>) -> Node {
    let mut read = read_of(role, name, &[]);
    read.node.traits.states = Field::Present(words(states));
    WalkedNode {
        payload: read,
        children,
        truncated: false,
        child_count: None,
        source_index: 0,
        in_web_document: false,
    }
}

pub(super) fn item(name: &str, showing: bool) -> Node {
    let states: &[u32] = if showing { &[SHOWING] } else { &[] };
    node(ITEM, name, states, Vec::new())
}

pub(super) fn items(showing: bool) -> Vec<Node> {
    vec![item("Open", showing), item("Save", showing)]
}

pub(super) fn gtk4_popover(showing: bool) -> Node {
    let scrolled = node(
        PANEL,
        "",
        &[SHOWING],
        vec![node(PANEL, "", &[SHOWING], items(showing))],
    );
    node(MENU, "Main Menu", &[SHOWING], vec![scrolled])
}

pub(super) fn frame_with(children: Vec<Node>) -> Node {
    node(FRAME, "App", &[SHOWING], children)
}

pub(super) fn gtk3_bar(showing: bool) -> Node {
    let menu = node(MENU, "File", &[SHOWING], items(showing));
    node(MENU_BAR, "", &[SHOWING], vec![menu])
}

pub(super) fn qt_bar(popup_showing: bool) -> Node {
    let states: &[u32] = if popup_showing { &[SHOWING] } else { &[] };
    let popup = node(POPUP_MENU, "app", states, items(popup_showing));
    let file = node(ITEM, "File", &[SHOWING], vec![popup]);
    node(MENU_BAR, "", &[SHOWING], vec![file])
}

pub(super) fn gtk3_context_window() -> Node {
    let menu = node(MENU, "", &[SHOWING], items(true));
    node(WINDOW, "", &[SHOWING], vec![menu])
}

pub(super) fn chromium_frame() -> Node {
    let menu = node(MENU, "", &[SHOWING], items(true));
    let bar = node(MENU_BAR, "", &[SHOWING], vec![menu]);
    node(FRAME, "", &[SHOWING], vec![bar])
}

pub(super) fn header_toggle(checked: bool) -> Node {
    let states: &[u32] = if checked {
        &[SHOWING, CHECKED]
    } else {
        &[SHOWING]
    };
    node(TOGGLE, "Menu", states, Vec::new())
}

pub(super) fn header_popover_panel(showing: bool) -> Node {
    let states: &[u32] = if showing { &[SHOWING] } else { &[] };
    let buttons = vec![
        node(BUTTON, "Zoom Out", states, Vec::new()),
        node(BUTTON, "New Window", states, Vec::new()),
    ];
    let inner = node(PANEL, "", states, buttons);
    let container: &[u32] = if showing { &[MODAL, SHOWING] } else { &[MODAL] };
    node(PANEL, "", container, vec![inner])
}
