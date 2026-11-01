//! One agent cursor's travel in flight: the shaped path from its pose to
//! the destination, when it started, the motion budget, the arrival frames
//! it waits to see shown, and the connection that waits for the arrival
//! acknowledgement. The renderer advances every
//! travel one frame at a time, so several agents' cursors travel at once.

use std::os::unix::net::UnixStream;
use std::time::Instant;

use agent_desktop_core::{AdapterError, CursorMotion, Deadline, Point};

use super::agent_cursor::AgentCursor;

const ARRIVAL_SHOW_MS: u64 = 120;

/// A travel that has not yet reached its arrival frame.
pub(crate) struct CursorTravel {
    motion: Option<CursorMotion>,
    started: Instant,
    budget: Deadline,
    to: Point,
    reply: UnixStream,
    arrival: Option<((u32, u32), Deadline)>,
}

impl CursorTravel {
    /// Starts `cursor` towards `to`, arriving by `budget` at the latest;
    /// `reply` gets the acknowledgement. A cursor with no pose arrives on
    /// its first frame.
    pub(crate) fn begin(
        cursor: &mut AgentCursor,
        to: Point,
        click: bool,
        budget: Deadline,
        reply: UnixStream,
    ) -> Self {
        let motion = cursor.pose.clone().map(|from| {
            CursorMotion::shaped(from, to.clone(), &cursor.motion, cursor.moves)
                .with_impact(click)
                .with_ripple(cursor.style.ripple())
        });
        cursor.moves = cursor.moves.wrapping_add(1);
        Self {
            motion,
            started: Instant::now(),
            budget,
            to,
            reply,
            arrival: None,
        }
    }

    /// The point to draw now, and whether it is the arrival frame: the
    /// destination once the path's time or the motion budget is spent.
    pub(crate) fn frame(&self) -> (Point, bool) {
        let elapsed = u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX);
        match &self.motion {
            Some(motion) if elapsed < motion.duration_ms() && !self.budget.is_expired() => {
                (motion.sample(elapsed), false)
            }
            _ => (self.to.clone(), true),
        }
    }

    /// Records the arrival frames committed for this travel. They must show
    /// by the end of the motion budget plus a short allowance, which keeps
    /// the answer inside the arrival timeout the caller waits for.
    pub(crate) fn await_arrival(&mut self, frames: (u32, u32)) -> Result<(), AdapterError> {
        let wait_ms = self.budget.remaining_ms().saturating_add(ARRIVAL_SHOW_MS);
        self.arrival = Some((frames, Deadline::detached_after(wait_ms)?));
        Ok(())
    }

    /// Forgets the arrival frames, so the next step commits the arrival
    /// again; an output change destroys the surfaces they were on.
    pub(crate) fn recommit_arrival(&mut self) {
        self.arrival = None;
    }

    /// The arrival frames this travel waits to see shown, once committed.
    pub(crate) fn arrival_frames(&self) -> Option<(u32, u32)> {
        self.arrival.as_ref().map(|(frames, _)| *frames)
    }

    /// Whether the arrival frames had their time to show and did not.
    pub(crate) fn arrival_overdue(&self) -> bool {
        self.arrival
            .as_ref()
            .is_some_and(|(_, deadline)| deadline.is_expired())
    }

    /// The connection that waits for this travel's acknowledgement.
    pub(crate) fn into_reply(self) -> UnixStream {
        self.reply
    }
}
