//! The in-process Wayland client transport shared by the readers that speak
//! wlroots-family and `ext` protocols. Every connection goes to the socket
//! path the caller names, every read waits for at most the caller's deadline,
//! and no outside tool or compositor-specific IPC is involved.

pub(crate) mod dispatch;
pub(crate) mod display_outputs;
pub(crate) mod display_wayland;
pub(crate) mod exchange;
pub(crate) mod global;
pub(crate) mod link;
pub(crate) mod output_probe;
pub(crate) mod socket_connect;
pub(crate) mod wayland_fault;
pub(crate) mod wayland_output;

#[cfg(test)]
pub(crate) mod fake_compositor;
