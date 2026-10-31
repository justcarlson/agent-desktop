//! The bound compositor's foreign toplevels, each mapped to its Hyprland
//! address through `hyprland_toplevel_mapping_manager_v1`.

pub(crate) mod bound_socket;
#[cfg(test)]
pub(crate) mod fake_binding;
pub(crate) mod mapped_toplevels;
pub(crate) mod mapping_target;
pub(crate) mod toplevel_join;
pub(crate) mod toplevel_read;
pub(crate) mod toplevel_reader;
pub(crate) mod toplevel_session;
pub(crate) mod toplevel_view;
pub(crate) mod wlr_window;
