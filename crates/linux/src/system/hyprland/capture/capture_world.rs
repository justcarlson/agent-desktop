use agent_desktop_core::{AdapterError, DisplayInfo};

use crate::system::wayland::capture::raw_frame::RawFrame;

/// What Hyprland capture reads from one bound compositor. Every read is
/// fresh, and each capture asks for one new frame.
pub(crate) trait CaptureWorld {
    fn enforced(&mut self) -> Result<bool, AdapterError>;
    fn displays(&mut self) -> Result<Vec<DisplayInfo>, AdapterError>;
    fn capture_output(&mut self, name: &str) -> Result<RawFrame, AdapterError>;
}
