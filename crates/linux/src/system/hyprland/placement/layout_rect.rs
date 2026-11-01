use agent_desktop_core::Rect;

use super::bound_layout::BoundLayout;
use super::placement::Placement;

/// Places `window_rect`, an element's rectangle relative to its toplevel,
/// in global layout logical pixels. Hyprland reports a client's `at` and
/// every output's origin in that space already, so no output scale applies.
pub(crate) fn layout_rect(bound: &BoundLayout<'_>, window_rect: &Rect) -> Placement {
    Placement {
        rect: *window_rect,
        output: None,
        shown: bound.client.shown,
    }
}

#[cfg(test)]
#[path = "layout_rect_tests.rs"]
mod tests;
