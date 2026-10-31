use agent_desktop_core::Rect;

use super::client_workspace::ClientWorkspace;

/// One Hyprland client from `j/clients`: the toplevel's compositor address,
/// its owning process, title, class, layout geometry and workspace.
/// `shown` is false for a client that is unmapped or hidden.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HyprClient {
    pub(crate) address: u64,
    pub(crate) pid: u32,
    pub(crate) title: String,
    pub(crate) class: String,
    pub(crate) geometry: Rect,
    pub(crate) workspace: ClientWorkspace,
    pub(crate) shown: bool,
}
