//! Which compositor toplevel a window is: an accessibility frame bound to
//! exactly one Hyprland client by evidence, or a compositor-only toplevel
//! named by its foreign toplevel identifier.

pub(crate) mod compositor_window_id;
pub(crate) mod compositor_windows;
pub(crate) mod identity_bind;
pub(crate) mod identity_refusal;
pub(crate) mod inventory;
pub(crate) mod inventory_read;
pub(crate) mod window_evidence;
