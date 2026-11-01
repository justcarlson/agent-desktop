//! Hyprland lists only toplevel windows, so a node drawn in a popup surface
//! (FINDINGS L32-11: GTK 4 popover extents are local to the popup) has no
//! compositor origin to place it by.

use agent_desktop_core::AdapterError;

use crate::system::hyprland::receiver::receiver_refusal::ReceiverRefusal;
use crate::tree::vocabulary::role_numbers::{MENU, PANEL, POPUP_MENU};

/// The refusal for a node whose frame is a popup surface: a `menu` or
/// popup menu frame, or a `panel` frame reached from a menu target. Any other frame is a
/// toplevel and answers `None`.
pub(crate) fn popup_surface_refusal(role: Option<u32>, menu_target: bool) -> Option<AdapterError> {
    (matches!(role, Some(MENU | POPUP_MENU)) || menu_target && role == Some(PANEL))
        .then(|| ReceiverRefusal::PopupSurface.to_error())
}

#[cfg(test)]
#[path = "popup_surface_tests.rs"]
mod tests;
