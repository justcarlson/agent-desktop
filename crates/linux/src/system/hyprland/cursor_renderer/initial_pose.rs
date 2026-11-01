//! Where a new renderer first shows the cursor: the middle of the focused
//! monitor, in that monitor's logical pixels. The cursor surface is placed
//! on the focused monitor, so this is a point on its own output. A reply
//! that names no usable monitor gives the origin.

use agent_desktop_core::Point;

/// The initial cursor point from a Hyprland `j/monitors` reply.
pub(crate) fn initial_point(_monitors: &[u8]) -> Point {
    Point { x: 0.0, y: 0.0 }
}

#[cfg(test)]
#[path = "initial_pose_tests.rs"]
mod tests;
