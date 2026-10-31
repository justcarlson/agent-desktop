//! The in-process Wayland client transport shared by the readers that speak
//! wlroots-family and `ext` protocols. Every connection goes to the socket
//! path the caller names, every read waits for at most the caller's deadline,
//! and no outside tool or compositor-specific IPC is involved.
//!
//! The default build speaks the protocol in pure Rust. The opt-in
//! `wayland-dlopen` feature switches the backend to the system
//! libwayland-client, loaded with `dlopen` at the first connection. When
//! `libwayland-client.so.0` cannot be loaded, the command fails with
//! `PLATFORM_NOT_SUPPORTED` and a platform detail that names the library.
//! Its size and linkage cost is in the Linux skill's
//! `references/display-servers-and-input.md`.

pub(crate) mod dispatch;
pub(crate) mod exchange;
pub(crate) mod global;
pub(crate) mod link;
pub(crate) mod socket_connect;
pub(crate) mod wayland_fault;

#[cfg(test)]
pub(crate) mod fake_compositor;
