use crate::tree::role_numbers::is_web_document;
use crate::tree::role_numbers::{
    CHECK_MENU_ITEM, FILLER, FRAME, PANEL, PUSH_BUTTON, RADIO_MENU_ITEM, TOGGLE_BUTTON, WINDOW,
};
use crate::tree::state_bits::{CHECKED, MODAL, SHOWING, StateBits};
use crate::tree::{ObservedRead, WalkedNode};

pub(super) type Node = WalkedNode<ObservedRead>;

const MENU: u32 = 33;
const MENU_BAR: u32 = 34;
const MENU_ITEM: u32 = 35;
const POPUP_MENU: u32 = 41;
const SEPARATOR: u32 = 50;
const ITEM_SEARCH_DEPTH: usize = 12;

/// Where a popup lives in the tree, which decides how its root is found. The
/// family is read from the placement the tree shows, never from the toolkit's
/// name: one toolkit has both an in-window and a top-level placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuFamily {
    Gtk3Bar,
    Gtk3Context,
    Gtk4Popover,
    Gtk3Popover,
    Qt,
    Chromium,
}

impl MenuFamily {
    /// Whether popups of the family are top-level windows of the application
    /// rather than children of the window that opened them.
    pub(crate) fn is_top_level(self) -> bool {
        matches!(self, Self::Gtk3Context | Self::Chromium)
    }
}

pub(super) struct Found<'a> {
    pub(super) root: &'a Node,
    pub(super) family: MenuFamily,
    pub(super) items: usize,
}

fn role(node: &Node) -> Option<u32> {
    node.payload.node.traits.role.present().copied()
}

fn has_state(node: &Node, bit: u32) -> bool {
    node.payload
        .node
        .traits
        .states
        .present()
        .is_some_and(|words| StateBits::new(words).has(bit))
}

fn is_item(role: Option<u32>) -> bool {
    matches!(role, Some(MENU_ITEM | CHECK_MENU_ITEM | RADIO_MENU_ITEM))
}

fn is_menu(role: Option<u32>) -> bool {
    matches!(role, Some(MENU | POPUP_MENU))
}

fn showing_items(node: &Node, depth: usize) -> usize {
    if depth > ITEM_SEARCH_DEPTH {
        return 0;
    }
    node.children
        .iter()
        .map(|child| {
            let child_role = role(child);
            if is_item(child_role) {
                usize::from(has_state(child, SHOWING)) + showing_items(child, depth + 1)
            } else if is_menu(child_role) {
                0
            } else {
                showing_items(child, depth + 1)
            }
        })
        .sum()
}

fn open_menu(node: &Node) -> Option<usize> {
    let open = is_menu(role(node)) && has_state(node, SHOWING);
    open.then(|| showing_items(node, 0))
        .filter(|items| *items > 0)
}

fn collect<'a>(node: &'a Node, in_bar: bool, found: &mut Vec<Found<'a>>) {
    if node.in_web_document || is_web_document(role(node)) {
        return;
    }
    let kind = role(node);
    if let Some(items) = open_menu(node) {
        let family = match kind {
            Some(POPUP_MENU) => MenuFamily::Qt,
            _ if in_bar => MenuFamily::Gtk3Bar,
            _ => MenuFamily::Gtk4Popover,
        };
        found.push(Found {
            root: node,
            family,
            items,
        });
    }
    let in_bar = in_bar || kind == Some(MENU_BAR);
    for child in &node.children {
        collect(child, in_bar, found);
    }
}

fn showing_buttons(node: &Node, depth: usize) -> usize {
    if depth > ITEM_SEARCH_DEPTH || node.in_web_document {
        return 0;
    }
    node.children
        .iter()
        .map(|child| {
            let own = role(child) == Some(PUSH_BUTTON) && has_state(child, SHOWING);
            usize::from(own) + showing_buttons(child, depth + 1)
        })
        .sum()
}

fn has_checked_toggle(node: &Node) -> bool {
    if node.in_web_document || is_web_document(role(node)) {
        return false;
    }
    let checked = role(node) == Some(TOGGLE_BUTTON) && has_state(node, CHECKED);
    checked || node.children.iter().any(has_checked_toggle)
}

fn header_popover(node: &Node) -> Option<(&Node, usize)> {
    if node.in_web_document || is_web_document(role(node)) {
        return None;
    }
    let container = role(node) == Some(PANEL) && has_state(node, MODAL) && has_state(node, SHOWING);
    if container {
        let buttons = showing_buttons(node, 0);
        if buttons > 0 {
            return Some((node, buttons));
        }
    }
    node.children.iter().find_map(header_popover)
}

/// The header-bar popover of GTK 3: a checked toggle in the window and a
/// modal, showing panel with showing buttons. The panel stays in the tree
/// while the popover is closed, so the toggle is what tells a closed popover
/// from an open one, and the panel is what tells it from a toggle that is
/// checked for another reason. A modal panel without the toggle is a dialog.
fn gtk3_popover(frame: &Node) -> Option<Found<'_>> {
    if !has_checked_toggle(frame) {
        return None;
    }
    let (root, items) = header_popover(frame)?;
    Some(Found {
        root,
        family: MenuFamily::Gtk3Popover,
        items,
    })
}

/// The popups open inside one window's tree, outermost first. A menu is open
/// when it shows and one of its items shows: a closed Qt popup and a closed
/// GTK 3 menu keep their nodes with the items cleared, and the menu-bar items
/// of every toolkit show while idle, so neither node presence nor a showing
/// bar item says anything.
pub(super) fn in_window(frame: &Node) -> Vec<Found<'_>> {
    let mut found = Vec::new();
    collect(frame, false, &mut found);
    if found.is_empty() {
        found.extend(gtk3_popover(frame));
    }
    found
}

fn is_popup_content(role: Option<u32>) -> bool {
    matches!(
        role,
        Some(
            MENU | MENU_BAR
                | MENU_ITEM
                | POPUP_MENU
                | SEPARATOR
                | CHECK_MENU_ITEM
                | RADIO_MENU_ITEM
                | PANEL
                | FILLER
        )
    )
}

pub(super) fn only_menu_content(node: &Node) -> bool {
    is_popup_content(role(node)) && node.children.iter().all(only_menu_content)
}

fn first_open_menu(node: &Node) -> Option<(&Node, usize)> {
    if let Some(items) = open_menu(node) {
        return Some((node, items));
    }
    node.children.iter().find_map(first_open_menu)
}

/// The popup a top-level child of the application root is, when it is one: a
/// nameless showing window or frame whose whole subtree is menu content.
/// GTK 3 context menus are such a window and Chromium menus such a frame; a
/// real window has a title or holds something besides menus.
pub(super) fn top_level(window: &Node) -> Option<Found<'_>> {
    let family = match role(window) {
        Some(WINDOW) => MenuFamily::Gtk3Context,
        Some(FRAME) => MenuFamily::Chromium,
        _ => return None,
    };
    let nameless = window.payload.node.properties.name.is_blank();
    if !nameless || !has_state(window, SHOWING) {
        return None;
    }
    if !window.children.iter().all(only_menu_content) {
        return None;
    }
    let (root, items) = window.children.iter().find_map(first_open_menu)?;
    Some(Found {
        root,
        family,
        items,
    })
}

#[cfg(test)]
#[path = "menu_popup_tests.rs"]
mod tests;
