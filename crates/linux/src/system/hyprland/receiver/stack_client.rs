use agent_desktop_core::Rect;

use super::stacking::Stacking;
use crate::system::hyprland::clients::client_workspace::ClientWorkspace;

/// One client from `j/clients` with what decides whether it can cover
/// another: floating, pinned and fullscreen state, and whether it is shown.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StackClient {
    pub(crate) address: u64,
    pub(crate) rect: Rect,
    pub(crate) workspace: ClientWorkspace,
    pub(crate) stacking: Stacking,
    pub(crate) shown: bool,
}
