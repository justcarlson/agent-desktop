use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::wayland::toplevel::toplevel_view::ToplevelView;

/// One read of the bound compositor: its clients over IPC and its
/// toplevels over the foreign toplevel protocols.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Inventory<'a> {
    pub(crate) clients: &'a [HyprClient],
    pub(crate) toplevels: &'a [ToplevelView],
}
