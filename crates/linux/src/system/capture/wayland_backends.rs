use agent_desktop_core::{AdapterError, Deadline, ImageBuffer, ScreenshotTarget};

use crate::system::hyprland::hyprland_binding::HyprlandBinding;

pub(crate) type PortalCapture<'a> =
    &'a dyn Fn(&ScreenshotTarget, Deadline) -> Result<ImageBuffer, AdapterError>;

pub(crate) type HyprlandCapture<'a> =
    &'a dyn Fn(&HyprlandBinding, &ScreenshotTarget, Deadline) -> Result<ImageBuffer, AdapterError>;

/// How a Wayland session captures: a generic compositor through the
/// screenshot portal, a bound Hyprland through its own capture protocols.
/// The two never stand in for each other.
pub(crate) struct WaylandBackends<'a> {
    pub(crate) portal: PortalCapture<'a>,
    pub(crate) hyprland: HyprlandCapture<'a>,
}
