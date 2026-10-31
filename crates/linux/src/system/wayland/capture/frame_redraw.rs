//! How a toplevel capture gets a frame drawn after the request. Hyprland
//! composes a toplevel that is not in view from the buffer it drew when it
//! was last shown or captured, which can be seconds old, and asks it to draw
//! again only while it composes a frame of it. Hyprland serves the first
//! frame of a session at once, but holds a later frame of the same session
//! until the output repaints for some other reason, which an idle desktop
//! never does, and it tells a capture client nothing when the toplevel
//! commits: the damage covers the whole buffer and the presentation time is
//! the composition's, on both Hyprland releases Omarchy ships.
//!
//! So a capture takes the first frame of one session to make the toplevel
//! redraw, then opens a new session at each redraw pause and returns the
//! first frame whose pixels differ from that first frame: those pixels were
//! drawn after the request. A toplevel whose pixels stay the same for
//! `REDRAW_POLLS` pauses drew nothing new in that time, and its unchanged
//! frame is the answer. A drawing that lands later is not waited for.

use std::time::Duration;

use agent_desktop_core::Deadline;

use super::capture_fault::CaptureFault;
use super::frame_failure::FrameFailure;
use super::raw_frame::RawFrame;
use crate::system::wayland::wayland_fault::WaylandFault;

/// The pause between sessions while a capture waits for the redraw.
pub(crate) const REDRAW_SETTLE: Duration = Duration::from_millis(25);

/// How many unchanged frames after the first end the wait for a redraw. At
/// one per `REDRAW_SETTLE`, a toplevel has at least 250 ms to draw.
pub(crate) const REDRAW_POLLS: usize = 10;

/// What a capture leaves of the command for the checks after it. With less
/// left before the redraw wait is over, nothing shows the toplevel drew
/// nothing new, so the capture times out.
pub(crate) const REDRAW_RESERVE: Duration = Duration::from_millis(100);

/// The pause before a new session when the compositor refused the buffer:
/// a Hyprland release can announce a moved toplevel's size before it settles.
pub(crate) const CONSTRAINTS_PAUSE: Duration = Duration::from_millis(50);

/// How many refused buffers a capture retries. A refusal the deadline leaves
/// no time to retry is a timeout, not a refusal.
pub(crate) const CONSTRAINTS_RETRIES: usize = 40;

/// How long a capture pauses before its next session with `remaining` left
/// of the command: the redraw pause, but never more than a quarter of what
/// is left.
pub(crate) fn redraw_wait(remaining: Duration) -> Duration {
    REDRAW_SETTLE.min(remaining / 4)
}

/// Takes frames through `take`, each the first frame of a new session. The
/// first frame is never the answer. The answer is the first later frame that
/// differs from it, or the last of `REDRAW_POLLS` unchanged ones. `pause`
/// waits without sending anything.
pub(crate) fn capture_after_redraw(
    deadline: Deadline,
    mut take: impl FnMut(Deadline) -> Result<RawFrame, CaptureFault>,
    mut pause: impl FnMut(Duration),
) -> Result<RawFrame, CaptureFault> {
    let mut first: Option<RawFrame> = None;
    let mut unchanged = 0;
    let mut refusals = 0;
    loop {
        if first.is_some() && deadline.remaining() <= REDRAW_RESERVE {
            return Err(CaptureFault::FrameSilent);
        }
        match take(deadline) {
            Ok(frame) => match &first {
                None => {
                    first = Some(frame);
                    pause(redraw_wait(deadline.remaining()));
                }
                Some(earlier) if frame != *earlier => return Ok(frame),
                Some(_) => {
                    unchanged += 1;
                    if unchanged >= REDRAW_POLLS {
                        return Ok(frame);
                    }
                    pause(redraw_wait(deadline.remaining()));
                }
            },
            Err(CaptureFault::Failed(FrameFailure::BufferConstraints))
                if refusals < CONSTRAINTS_RETRIES =>
            {
                if deadline.remaining() <= CONSTRAINTS_PAUSE {
                    return Err(CaptureFault::FrameSilent);
                }
                refusals += 1;
                pause(CONSTRAINTS_PAUSE);
            }
            Err(CaptureFault::Setup(WaylandFault::Silent)) if first.is_some() => {
                return Err(CaptureFault::FrameSilent);
            }
            Err(fault) => return Err(fault),
        }
    }
}

#[cfg(test)]
#[path = "frame_redraw_tests.rs"]
mod tests;
