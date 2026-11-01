use crate::input::hyprland::hypr_route::bound_route;
use crate::system::display::display_server::{DisplayServer, session_server};
use crate::tree::properties::node_read::COMPONENT_INTERFACE;
use crate::tree::vocabulary::role_numbers::{
    DOCUMENT_FRAME, DOCUMENT_WEB, LIST, SCROLL_PANE, VIEWPORT,
};

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
/// pointer leg.
pub(crate) fn eligible(role: u32, interfaces: &[String], pointer_leg: bool) -> bool {
    pointer_leg && wheel_target(role, interfaces)
}

/// Whether this process's session has a pointer leg for a wheel.
pub(crate) fn session_pointer_leg() -> bool {
    pointer_leg(&session_server(), || matches!(bound_route(), Ok(Some(_))))
}

/// Whether `session` has a pointer leg for a wheel: an X11 display has one.
pub(crate) fn pointer_leg(session: &DisplayServer, _hyprland_bound: impl FnOnce() -> bool) -> bool {
    matches!(session, DisplayServer::X11(_))
}

/// The part of eligibility a node decides alone: a role that moves content
/// under a wheel and a `Component` interface to take bounds from.
pub(crate) fn wheel_target(role: u32, interfaces: &[String]) -> bool {
    WHEEL_TARGETS.contains(&role) && interfaces.iter().any(|name| name == COMPONENT_INTERFACE)
}

#[cfg(test)]
#[path = "scroll_capability_tests.rs"]
mod tests;
