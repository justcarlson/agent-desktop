//! The in-process Wayland client transport shared by the readers that speak
//! wlroots-family and `ext` protocols. Every connection goes to the socket
//! path the caller names, every read waits for at most the caller's deadline,
//! and no outside tool or compositor-specific IPC is involved.

pub(crate) mod dispatch;
pub(crate) mod exchange;
pub(crate) mod ext_toplevel;
pub(crate) mod global;
pub(crate) mod link;
pub(crate) mod registry_read;
pub(crate) mod socket_connect;
pub(crate) mod socket_peer;
pub(crate) mod toplevel_list;
pub(crate) mod wayland_fault;
pub(crate) mod wlr_toplevel;
pub(crate) mod wlr_toplevels;

#[cfg(test)]
pub(crate) mod fake_compositor;
#[cfg(test)]
pub(crate) mod fake_toplevels;
