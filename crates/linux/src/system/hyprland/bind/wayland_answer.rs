/// What one Wayland socket did for an expected compositor pid. `Other` names
/// a socket served by a different process and is decided before any read.
/// `Live` answered a registry round trip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WaylandAnswer {
    Live,
    Other(u32),
    Gone,
    Silent,
}
