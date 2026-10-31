use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::wayland::toplevel::toplevel_view::ToplevelView;

/// The one compositor toplevel a window was bound to: its client record and
/// its foreign toplevel handles.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BoundToplevel {
    pub(crate) client: HyprClient,
    pub(crate) toplevel: ToplevelView,
}
