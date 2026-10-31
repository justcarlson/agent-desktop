//! How a toplevel capture gets a frame drawn after the request. Hyprland
//! composes a toplevel that is not in view from the buffer it drew when it
//! was last shown or captured, and asks it to draw again only while it
//! composes a frame of it. Hyprland serves the first frame of a session at
//! once, but holds a later frame of the same session until the output
//! repaints for some other reason, which an idle desktop never does. So a
//! capture takes the first frame of one session to make the toplevel
//! redraw, waits for the redraw, and returns the first frame of a new
//! session. A toplevel with nothing new to draw gives its unchanged content.

use std::time::Duration;

use agent_desktop_core::Deadline;

use super::capture_fault::CaptureFault;
use super::frame_failure::FrameFailure;
use super::raw_frame::RawFrame;
use crate::system::wayland::wayland_fault::WaylandFault;

/// How long a capture lets the toplevel draw after the first frame asked
/// it to.
pub(crate) const REDRAW_SETTLE: Duration = Duration::from_millis(25);

/// The pause before a new session when the compositor refused the buffer:
/// a Hyprland release can announce a moved toplevel's size before it settles.
pub(crate) const CONSTRAINTS_PAUSE: Duration = Duration::from_millis(50);

/// How many refused buffers a capture retries.
pub(crate) const CONSTRAINTS_RETRIES: usize = 40;

/// How long the redraw is awaited with `remaining` left of the command:
/// the settle window, but never more than a quarter of what is left, so
/// the second session and the checks after it still have time to run.
pub(crate) fn redraw_wait(remaining: Duration) -> Duration {
    REDRAW_SETTLE.min(remaining / 4)
}

/// Takes frames through `take`, each the first frame of a new session,
/// until one arrives after an earlier frame and the redraw wait. `pause`
/// waits without sending anything. The first frame is never the answer.
pub(crate) fn capture_after_redraw(
    deadline: Deadline,
    mut take: impl FnMut(Deadline) -> Result<RawFrame, CaptureFault>,
    mut pause: impl FnMut(Duration),
) -> Result<RawFrame, CaptureFault> {
    let mut primed = false;
    let mut refusals = 0;
    loop {
        match take(deadline) {
            Ok(frame) if primed => return Ok(frame),
            Ok(_) => {
                primed = true;
                pause(redraw_wait(deadline.remaining()));
            }
            Err(CaptureFault::Failed(FrameFailure::BufferConstraints))
                if refusals < CONSTRAINTS_RETRIES && deadline.remaining() > CONSTRAINTS_PAUSE =>
            {
                refusals += 1;
                pause(CONSTRAINTS_PAUSE);
            }
            Err(CaptureFault::Setup(WaylandFault::Silent)) if primed => {
                return Err(CaptureFault::FrameSilent);
            }
            Err(fault) => return Err(fault),
        }
    }
}

#[cfg(test)]
#[path = "frame_redraw_tests.rs"]
mod tests;
