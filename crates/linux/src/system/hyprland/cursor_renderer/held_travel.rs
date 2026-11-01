//! Travels while a holder hides the cursors. Each travel keeps its schedule
//! and its pose but draws nothing, and is acknowledged at its arrival time,
//! so another command's travel finishes on time instead of drawing over a
//! capture or headed input. A `show` then draws each cursor where it landed.

use std::os::unix::net::UnixStream;

use super::agent_cursors::AgentCursors;
use super::cursor_travel::CursorTravel;

/// How often the renderer advances unseen travels; nothing is drawn, so
/// there is no frame to wait for.
pub(crate) const UNSEEN_STEP_MS: u64 = 16;

/// Advances every travel without drawing and returns the connections of
/// the travels that reached their arrival time.
pub(crate) fn step_unseen(cursors: &mut AgentCursors) -> Vec<UnixStream> {
    let mut arrived = Vec::new();
    for (_, cursor) in cursors.iter_mut() {
        let Some((point, done)) = cursor.travel.as_ref().map(CursorTravel::frame) else {
            continue;
        };
        cursor.pose = Some(point);
        if done && let Some(travel) = cursor.travel.take() {
            arrived.push(travel.into_reply());
        }
    }
    arrived
}

#[cfg(test)]
#[path = "held_travel_tests.rs"]
mod tests;
