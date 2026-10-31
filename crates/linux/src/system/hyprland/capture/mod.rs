//! `screenshot` on Hyprland: a display from its output on the compositor's
//! own capture protocol, refused up front when Hyprland enforces
//! permissions.

pub(crate) mod capture_entry;
pub(crate) mod capture_error;
pub(crate) mod capture_world;
pub(crate) mod cursor_hider;
pub(crate) mod enforce_option;
pub(crate) mod frame_image;
pub(crate) mod hypr_capture;
pub(crate) mod live_capture_world;
pub(crate) mod no_cursor_hider;
