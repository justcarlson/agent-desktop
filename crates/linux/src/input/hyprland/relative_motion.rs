//! The relative virtual pointer motions that carry the cursor to an aim, and
//! the check that it landed there before any button or wheel event.
//!
//! Hyprland forwards an absolute virtual pointer motion with no relative
//! motion and closes a frame after every motion, so XWayland moves its
//! absolute pointer device for it while it sends the wheel through its
//! relative one. The switch makes GDK3 reset its scroll baseline, and the
//! next notch scrolls nothing. A relative motion reaches XWayland with its
//! `wl_pointer.motion` in one frame, on the wheel's device
//! (probes/linux/FINDINGS.md L44-1).

use agent_desktop_core::{AdapterError, Point};

/// How far from the aim, in logical pixels on each axis, the cursor may
/// read and still count as on it.
#[allow(dead_code)]
const LANDING_TOLERANCE: f64 = 0.5;

/// The relative motions from `from` to `aim`, in logical pixels. A `first`
/// motion passes one pixel beside the aim, toward `beside_dx`, so Hyprland
/// hit-tests the aim even when the cursor already rests on it. A motion of
/// zero pixels is left out.
pub(crate) fn relative_moves(
    _from: &Point,
    _aim: (i32, i32),
    _first: bool,
    _beside_dx: i32,
) -> Vec<(f64, f64)> {
    Vec::new()
}

/// Whether `landed`, the cursor the compositor reports after the motions,
/// is on `aim`. A cursor off it refuses the run before any button or wheel
/// event.
pub(crate) fn confirm_landed(_landed: &Point, _aim: (i32, i32)) -> Result<(), AdapterError> {
    Ok(())
}

/// Marks a failed cursor read as missing landing evidence: the run refuses
/// before any button or wheel event.
pub(crate) fn no_cursor_evidence(error: AdapterError) -> AdapterError {
    error
}

#[cfg(test)]
#[path = "relative_motion_tests.rs"]
mod tests;
