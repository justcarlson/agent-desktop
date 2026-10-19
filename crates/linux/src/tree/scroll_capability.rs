#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the module is laid out ahead of the code that calls it"
    )
)]

use super::node_read::COMPONENT_INTERFACE;
use super::role_numbers::{DOCUMENT_FRAME, DOCUMENT_WEB, LIST, SCROLL_PANE, VIEWPORT};
use crate::system::display_server::DisplayServer;

const TABLE: u32 = 55;
const TREE: u32 = 65;
const TREE_TABLE: u32 = 66;

const WHEEL_TARGETS: [u32; 8] = [
    SCROLL_PANE,
    VIEWPORT,
    LIST,
    TABLE,
    TREE,
    TREE_TABLE,
    DOCUMENT_FRAME,
    DOCUMENT_WEB,
];

/// Whether a node can take a wheel at its centre as a scroll: its role moves
/// content under a wheel, it has bounds to aim at, and the session has a
/// pointer leg. A Wayland session has none, so no node is eligible there.
pub(crate) fn eligible(role: u32, interfaces: &[String], session: &DisplayServer) -> bool {
    matches!(session, DisplayServer::X11(_))
        && WHEEL_TARGETS.contains(&role)
        && interfaces.iter().any(|name| name == COMPONENT_INTERFACE)
}

#[cfg(test)]
#[path = "scroll_capability_tests.rs"]
mod tests;
