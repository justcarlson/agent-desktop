use agent_desktop_core::{AdapterError, Deadline, ImageBuffer, ScreenshotTarget};

use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;

pub(crate) type BindingLookup<'a> = &'a dyn Fn() -> Result<Option<HyprlandBinding>, AdapterError>;

pub(crate) type PortalCapture<'a> =
    &'a dyn Fn(&ScreenshotTarget, Deadline) -> Result<ImageBuffer, AdapterError>;

pub(crate) type HyprlandCapture<'a> =
    &'a dyn Fn(&HyprlandBinding, &ScreenshotTarget, Deadline) -> Result<ImageBuffer, AdapterError>;

/// How a Wayland session captures: the bound Hyprland instance, when
/// `binding` finds one, through its own capture protocols, and any other
/// compositor through the screenshot portal. A refused binding is the
/// error, and the two backends never stand in for each other.
pub(crate) struct WaylandBackends<'a> {
    pub(crate) binding: BindingLookup<'a>,
    pub(crate) portal: PortalCapture<'a>,
    pub(crate) hyprland: HyprlandCapture<'a>,
}
