use agent_desktop_core::{AdapterError, Deadline, ImageBuffer, ScreenshotTarget};

use super::hypr_capture::{capture_screen, capture_window};
use super::live_capture_world::LiveCaptureWorld;
use super::no_cursor_hider::NoCursorHider;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;

/// Captures `target` from the compositor `binding` names.
pub(crate) fn capture_hyprland(
    binding: &HyprlandBinding,
    target: &ScreenshotTarget,
    deadline: Deadline,
) -> Result<ImageBuffer, AdapterError> {
    let mut world = LiveCaptureWorld::new(binding.clone(), deadline);
    match target {
        ScreenshotTarget::ExactWindow(window) => capture_window(&mut world, window),
        screen => capture_screen(&mut world, &mut NoCursorHider, screen),
    }
}
