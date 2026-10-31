use agent_desktop_core::{AdapterError, DisplayInfo, WindowInfo};

use crate::system::hyprland::identity::bound_toplevel::BoundToplevel;
use crate::system::wayland::capture::composition::Composition;
use crate::system::wayland::capture::raw_frame::RawFrame;

/// What Hyprland capture reads from one bound compositor. Every read is
/// fresh, and each capture asks for one new frame.
pub(crate) trait CaptureWorld {
    fn enforced(&mut self) -> Result<bool, AdapterError>;
    fn bind(&mut self, win: &WindowInfo) -> Result<BoundToplevel, AdapterError>;
    /// How the compositor composes the toplevel `bound` names now.
    fn composition(&mut self, bound: &BoundToplevel) -> Result<Composition, AdapterError>;
    fn capture_toplevel(
        &mut self,
        identifier: &str,
        composition: Composition,
    ) -> Result<RawFrame, AdapterError>;
    /// Whether the compositor still has the toplevel `bound` names, at the
    /// same address.
    fn still_bound(&mut self, bound: &BoundToplevel) -> Result<bool, AdapterError>;
    fn displays(&mut self) -> Result<Vec<DisplayInfo>, AdapterError>;
    fn capture_output(&mut self, name: &str) -> Result<RawFrame, AdapterError>;
}
