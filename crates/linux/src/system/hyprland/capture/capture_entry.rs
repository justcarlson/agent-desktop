use agent_desktop_core::{AdapterError, Deadline, ImageBuffer, ScreenshotTarget};

use super::hypr_capture::capture_screen;
use super::live_capture_world::LiveCaptureWorld;
use super::no_cursor_hider::NoCursorHider;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;

/// Captures the display `target` names from the compositor `binding` names.
pub(crate) fn capture_hyprland(
    binding: &HyprlandBinding,
    target: &ScreenshotTarget,
    deadline: Deadline,
) -> Result<ImageBuffer, AdapterError> {
    let mut world = LiveCaptureWorld::new(binding.clone(), deadline);
    capture_screen(&mut world, &mut NoCursorHider, target)
}
