//! The compositor's toplevels over the foreign toplevel protocols, mapped to
//! Hyprland addresses, and their activation.

pub(crate) mod advertised_global;
pub(crate) mod chosen_globals;
pub(crate) mod ext_record;
pub(crate) mod mapping_target;
pub(crate) mod toplevel_dispatch;
pub(crate) mod toplevel_join;
pub(crate) mod toplevel_session;
pub(crate) mod toplevel_state;
pub(crate) mod toplevel_view;
pub(crate) mod wlr_record;
