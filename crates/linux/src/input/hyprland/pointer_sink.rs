use std::time::Duration;

use agent_desktop_core::AdapterError;

use super::output_point::OutputPoint;
use super::wheel_axis::WheelAxis;

/// Where a Hyprland pointer run sends its events. Each event is one frame.
/// `recover` releases `buttons` after a run stopped, even past the run's
/// deadline, and says whether every release was sent.
pub(crate) trait PointerSink {
    fn motion(&mut self, point: &OutputPoint) -> Result<(), AdapterError>;
    fn button(&mut self, code: u32, down: bool) -> Result<(), AdapterError>;
    fn wheel(&mut self, axis: WheelAxis, discrete: i32) -> Result<(), AdapterError>;
    fn pause(&mut self, duration: Duration);
    fn recover(&mut self, buttons: &[u32]) -> bool;
}
