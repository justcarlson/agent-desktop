use agent_desktop_core::AdapterError;

use super::client_state::ClientState;
use crate::system::hyprland::clients::desktop_state::DesktopState;
use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::hyprland::toplevel::toplevel_view::ToplevelView;

/// What a Hyprland window operation reads from and sends to one bound
/// compositor. Each read is fresh. `request` sends one IPC request and
/// returns the reply text; a `dispatch` request starts the read-back
/// window that `wait_more` paces and says whether it has time left.
pub(crate) trait WindowWorld {
    fn inventory(&mut self) -> Result<(Vec<HyprClient>, Vec<ToplevelView>), AdapterError>;
    fn states(&mut self) -> Result<Vec<ClientState>, AdapterError>;
    fn desktop(&mut self) -> Result<DesktopState, AdapterError>;
    fn request(&mut self, command: &str) -> Result<String, AdapterError>;
    fn live_instance(&self, pid: u32) -> Option<String>;
    fn compositor(&self) -> (u32, u64);
    fn wait_more(&mut self) -> bool;
}
