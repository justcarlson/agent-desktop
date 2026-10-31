use agent_desktop_core::AdapterError;

use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::hyprland::toplevel::toplevel_view::ToplevelView;

/// The reads a window capture binds and rechecks its toplevel from.
pub(crate) trait ToplevelReads {
    /// The compositor's clients, read now.
    fn clients(&mut self) -> Result<Vec<HyprClient>, AdapterError>;
    /// The toplevels on the connection the capture sends its requests on.
    fn toplevels(&mut self) -> Result<Vec<ToplevelView>, AdapterError>;
    /// The toplevels read now, on a new connection.
    fn fresh_toplevels(&mut self) -> Result<Vec<ToplevelView>, AdapterError>;
    /// The instance token of process `pid` as it runs now.
    fn live_instance(&self, pid: u32) -> Option<String>;
    /// The bound compositor's pid and start time.
    fn compositor(&self) -> (u32, u64);
}
