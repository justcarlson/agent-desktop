//! One agent cursor's travel in flight: the shaped path from its pose to
//! the destination, when it started, the motion budget, and the connection
//! that waits for the arrival acknowledgement. The renderer advances every
//! travel one frame at a time, so several agents' cursors travel at once.

use std::os::unix::net::UnixStream;
use std::time::Instant;

use agent_desktop_core::{CursorMotion, Deadline, Point};

use super::agent_cursor::AgentCursor;

/// A travel that has not yet reached its arrival frame.
pub(crate) struct CursorTravel {
    motion: Option<CursorMotion>,
    started: Instant,
    budget: Deadline,
    to: Point,
    reply: UnixStream,
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

    /// The connection that waits for this travel's acknowledgement.
    pub(crate) fn into_reply(self) -> UnixStream {
        self.reply
    }
}
