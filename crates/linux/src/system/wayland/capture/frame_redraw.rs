//! Which of a session's two frames a toplevel capture returns. The first
//! frame is composed after the request, but a toplevel that is not in view
//! still shows the buffer it drew when it was last shown or captured:
//! Hyprland asks it to redraw only while composing that frame. The second
//! frame carries what it drew in answer.

use std::time::Duration;

use super::capture_fault::CaptureFault;
use super::raw_frame::RawFrame;

/// How long a capture waits for the toplevel's redraw after the first
/// frame. Hyprland withholds the second frame of a toplevel on a regular
/// workspace out of view that has nothing new to draw.
pub(crate) const REDRAW_SETTLE: Duration = Duration::from_millis(250);

/// The redrawn frame when it came. A second frame withheld until the
/// settle window closed means the toplevel drew nothing new, so the first
/// frame is its current content. Any other failure of the second frame
/// fails the capture.
pub(crate) fn redrawn(
    first: RawFrame,
    second: Result<RawFrame, CaptureFault>,
) -> Result<RawFrame, CaptureFault> {
    match second {
        Ok(frame) => Ok(frame),
        Err(CaptureFault::FrameSilent) => Ok(first),
        Err(fault) => Err(fault),
    }
}

#[cfg(test)]
#[path = "frame_redraw_tests.rs"]
mod tests;
