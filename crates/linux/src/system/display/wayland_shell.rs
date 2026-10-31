use crate::system::hyprland::hyprland_binding::HyprlandBinding;

/// Which compositor a Wayland route talks to. `Hyprland` carries the one
/// validated instance binding; every other Wayland session is `Generic` and
/// keeps the portal and toolkit paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WaylandShell {
    Generic,
    Hyprland(HyprlandBinding),
}
