//! Drawing cursors. Every travelling cursor advances one frame per step:
//! each moves its own surface to the next point of its shaped path, then
//! the step waits once for every frame it committed, on every output, at
//! most one frame interval, so several agents' cursors travel at the same
//! time. A travel's last frame lands on its destination. Its connection
//! gets the acknowledgement only once every output that frame was
//! committed on has shown it, and a refusal if that does not happen by the
//! end of its motion budget plus a short allowance: a frame callback is the
//! only evidence an output drew the arrival, and the overlay layer has no
//! hidden or occluded state, because a hidden target parks the cursor and
//! commits no arrival frame.

use std::os::unix::net::UnixStream;

use agent_desktop_core::{AdapterError, Deadline, Point};

use super::agent_cursor::AgentCursor;
use super::agent_cursors::AgentCursors;
use super::cursor_travel::CursorTravel;
use super::layer_client::LayerClient;
use super::renderer_reply::arrival_unconfirmed;

const FRAME_WAIT_MS: u64 = 50;

/// Advances every travel one frame and returns the connections of the
/// travels whose arrival frame has shown; travels whose arrival frame is
/// overdue are refused here. On an error every travel stays in flight, so
/// the caller can answer each one.
pub(crate) fn step_travels(
    client: &mut LayerClient,
    cursors: &mut AgentCursors,
) -> Result<Vec<UnixStream>, AdapterError> {
    let deadline = Deadline::detached_after(FRAME_WAIT_MS)?;
    let mut oldest: Option<u32> = None;
    for (key, cursor) in cursors.iter_mut() {
        let Some(travel) = &cursor.travel else {
            continue;
        };
        if let Some((after, _)) = travel.arrival_frames() {
            oldest = Some(older(client.last_frame(), oldest, after));
            continue;
        }
        let (point, done) = travel.frame();
        let staged = stage(client, key, cursor, point, deadline)?;
        oldest = Some(older(client.last_frame(), oldest, staged.0));
        if let Some(travel) = cursor.travel.as_mut().filter(|_| done) {
            travel.await_arrival(staged)?;
        }
    }
    if let Some(after) = oldest {
        client.settle((after, client.last_frame()), deadline)?;
    }
    let mut arrived = Vec::new();
    for (_, cursor) in cursors.iter_mut() {
        let Some(frames) = cursor
            .travel
            .as_ref()
            .and_then(CursorTravel::arrival_frames)
        else {
            continue;
        };
        if client.shown(frames) {
            arrived.extend(cursor.travel.take().map(CursorTravel::into_reply));
        } else if cursor
            .travel
            .as_ref()
            .is_some_and(CursorTravel::arrival_overdue)
        {
            if let Some(travel) = cursor.travel.take() {
                arrival_unconfirmed(&travel.into_reply());
            }
        }
    }
    Ok(arrived)
}

fn older(last: u32, oldest: Option<u32>, after: u32) -> u32 {
    match oldest {
        Some(kept) if last.wrapping_sub(kept) >= last.wrapping_sub(after) => kept,
        _ => after,
    }
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
