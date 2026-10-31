/// Why a Wayland exchange ended without an answer: the compositor closed or
/// broke the connection, or it did not answer before the deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WaylandFault {
    Gone,
    Silent,
}
