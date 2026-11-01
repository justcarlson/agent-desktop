//! Hyprland lists only toplevel windows, so a node drawn in a popup surface
//! (FINDINGS L32-11: GTK 4 popover extents are local to the popup) has no
//! compositor origin to place it by.

use agent_desktop_core::AdapterError;

/// The refusal for a node whose frame is a popup surface: a `menu` frame,
/// or a `panel` frame reached from a menu target. Any other frame is a
/// toplevel and answers `None`.
pub(crate) fn popup_surface_refusal(
    _role: Option<u32>,
    _menu_target: bool,
) -> Option<AdapterError> {
    None
}

#[cfg(test)]
#[path = "popup_surface_tests.rs"]
mod tests;
