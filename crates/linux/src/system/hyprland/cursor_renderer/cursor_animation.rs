//! Drawing cursors. Every travelling cursor advances one frame per step:
//! each moves its own surface to the next point of its shaped path, then
//! the step waits once for every frame it committed, on every output, at
//! most one frame interval, so several agents' cursors travel at the same
//! time. A travel's
//! last frame lands on its destination, and its connection is handed back
//! for the acknowledgement only after that frame.

use std::os::unix::net::UnixStream;

use agent_desktop_core::{AdapterError, Deadline, Point};

use super::agent_cursor::AgentCursor;
use super::agent_cursors::AgentCursors;
use super::cursor_travel::CursorTravel;
use super::layer_client::LayerClient;

const FRAME_WAIT_MS: u64 = 50;

/// Advances every travel one frame and returns the connections of the
/// travels that drew their arrival frame. On an error every travel stays
/// in flight, so the caller can answer each one.
pub(crate) fn step_travels(
    client: &mut LayerClient,
    cursors: &mut AgentCursors,
) -> Result<Vec<UnixStream>, AdapterError> {
    let deadline = Deadline::detached_after(FRAME_WAIT_MS)?;
    let mut frames: Option<(u32, u32)> = None;
    let mut arrived = Vec::new();
    for (key, cursor) in cursors.iter_mut() {
        let Some(travel) = &cursor.travel else {
            continue;
        };
        let (point, done) = travel.frame();
        let staged = stage(client, key, cursor, point, deadline)?;
        frames = Some((frames.map_or(staged.0, |(after, _)| after), staged.1));
        if done {
            arrived.push(key.clone());
        }
    }
    if let Some(frames) = frames {
        client.settle(frames, deadline)?;
    }
    Ok(arrived
        .iter()
        .filter_map(|key| cursors.get_mut(key).and_then(|cursor| cursor.travel.take()))
        .map(CursorTravel::into_reply)
        .collect())
}

/// Shows the cursor under `key` at `point` and records it as the pose.
pub(crate) fn draw_at(
    client: &mut LayerClient,
    key: &str,
    cursor: &mut AgentCursor,
    point: Point,
) -> Result<(), AdapterError> {
    draw_within(client, (key, cursor), point, FRAME_WAIT_MS)
}

/// Like `draw_at`, giving the surface and its frame `wait_ms` to appear.
pub(crate) fn draw_within(
    client: &mut LayerClient,
    (key, cursor): (&str, &mut AgentCursor),
    point: Point,
    wait_ms: u64,
) -> Result<(), AdapterError> {
    let deadline = Deadline::detached_after(wait_ms)?;
    let frames = stage(client, key, cursor, point, deadline)?;
    client.settle(frames, deadline)
}

fn stage(
    client: &mut LayerClient,
    key: &str,
    cursor: &mut AgentCursor,
    point: Point,
    deadline: Deadline,
) -> Result<(u32, u32), AdapterError> {
    cursor.pose = Some(point.clone());
    client.stage_point(key, cursor, &point, deadline)
}
