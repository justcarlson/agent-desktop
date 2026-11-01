use agent_desktop_core::{Point, Rect};

use crate::input::physical::shell_aim::display_under;

use super::bound_layout::BoundLayout;
use super::layout_placement::Placement;

/// Places `window_rect`, an element's rectangle relative to its toplevel,
/// in global layout logical pixels. Hyprland reports a client's `at` and
/// every output's origin in that space already, so no output scale applies.
pub(crate) fn layout_rect(bound: &BoundLayout<'_>, window_rect: &Rect) -> Placement {
    let origin = &bound.client.geometry;
    let rect = Rect {
        x: origin.x + window_rect.x,
        y: origin.y + window_rect.y,
        width: window_rect.width,
        height: window_rect.height,
    };
    let centre = Point {
        x: rect.x + rect.width / 2.0,
        y: rect.y + rect.height / 2.0,
    };
    let output = display_under(&centre, bound.displays).map(|display| display.id.clone());
    let shown =
        bound.client.shown && bound.desktop.in_view(&bound.client.workspace) && output.is_some();
    Placement {
        rect,
        output,
        shown,
    }
}

#[cfg(test)]
#[path = "layout_rect_tests.rs"]
mod tests;
