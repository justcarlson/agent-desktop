//! The in-process Wayland transport. Every connection goes to an explicit
//! socket path that a validated binding names, never to the ambient
//! `WAYLAND_DISPLAY`, and every read carries a deadline.

pub(crate) mod bound_connection;
pub(crate) mod capture;
pub(crate) mod dispatch;
pub(crate) mod peer_fault;
pub(crate) mod protocol;
pub(crate) mod registry;
pub(crate) mod socket_peer;
pub(crate) mod toplevel;
pub(crate) mod wayland_fault;
