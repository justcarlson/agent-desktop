use agent_desktop_core::{AdapterError, WindowInfo};

use crate::system::hyprland::clients::desktop_state::DesktopState;
use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::hyprland::toplevel::toplevel_view::ToplevelView;

/// What Hyprland focus reads from and sends to one bound compositor. Each
/// read is fresh. `inventory` is the clients and toplevels that one read of
/// the compositor agrees on. `activated` is every address whose
/// `zwlr_foreign_toplevel_handle_v1` state holds `activated`; `wait_more`
/// paces the post-activation proof and says whether its budget has time
/// left.
pub(crate) trait HyprWorld {
    fn inventory(&mut self) -> Result<(Vec<HyprClient>, Vec<ToplevelView>), AdapterError>;
    fn activated(&mut self) -> Result<Vec<u64>, AdapterError>;
    fn desktop(&mut self) -> Result<DesktopState, AdapterError>;
    fn frame_active(&mut self, win: &WindowInfo) -> Result<bool, AdapterError>;
    fn live_instance(&self, pid: u32) -> Option<String>;
    fn compositor(&self) -> (u32, u64);
    fn activate(&mut self, address: u64) -> Result<(), AdapterError>;
    fn wait_more(&mut self) -> bool;
}
