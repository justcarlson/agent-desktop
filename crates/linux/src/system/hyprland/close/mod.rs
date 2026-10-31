//! `close-app` on Hyprland: a close request to every toplevel of the
//! process through the shared `zwlr_foreign_toplevel_handle_v1.close`, each
//! handle tied to its process by its Hyprland address.

pub(crate) mod compositor_close;
pub(crate) mod owned_toplevels;
