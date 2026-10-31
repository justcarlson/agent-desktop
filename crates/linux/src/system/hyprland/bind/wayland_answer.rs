/// What one Wayland socket did for an expected compositor pid. `Other` names
/// a socket served by a different process and is decided before any read.
/// `Live` answered a registry round trip and says whether it advertises the
/// Hyprland toplevel mapping global.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WaylandAnswer {
    Live { pid: u32, mapping: bool },
    Other(u32),
    Gone,
    Silent,
}
