use crate::system::hyprland::clients::desktop_state::DesktopState;

use super::layer_info::LayerInfo;
use super::stack_client::StackClient;

/// What the bound instance stacks at one read: every client, every layer
/// surface and what each monitor shows.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReceiverStack {
    pub(crate) clients: Vec<StackClient>,
    pub(crate) layers: Vec<LayerInfo>,
    pub(crate) desktop: DesktopState,
}
