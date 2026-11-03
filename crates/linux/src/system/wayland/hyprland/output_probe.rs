use crate::system::wayland::hyprland::wayland_fault::WaylandFault;

/// Why the Wayland output layout could not be read: the compositor does not
/// advertise the globals it needs, an output came back without a name,
/// position or size, or the connection itself failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OutputProbe {
    Unadvertised(Vec<&'static str>),
    Incomplete,
    Wayland(WaylandFault),
}
