//! Drawing a cursor's travel. Each frame moves the surface to the next point
//! of the shaped path and waits for the compositor's frame callback, at most
//! one frame interval; the last frame lands on the destination, and the
//! caller acknowledges travel only after it.

use std::time::Instant;

use agent_desktop_core::{AdapterError, CURSOR_MOTION_BUDGET_MS, CursorMotion, Deadline, Point};

use super::agent_cursor::AgentCursor;
use super::layer_client::LayerClient;
use super::placement::Placement;

const FRAME_WAIT_MS: u64 = 50;

/// Moves `cursor` to `to`, animating from its pose when it has one.
pub(crate) fn travel(
    client: &mut LayerClient,
    cursor: &mut AgentCursor,
    to: Point,
    click: bool,
) -> Result<(), AdapterError> {
    let Some(from) = cursor.pose.clone() else {
        return draw_at(client, cursor, to);
    };
    let motion = CursorMotion::shaped(from, to.clone(), &cursor.motion, cursor.moves)
        .with_impact(click)
        .with_ripple(cursor.style.ripple());
    cursor.moves = cursor.moves.wrapping_add(1);
    let started = Instant::now();
    let budget = Deadline::detached_after(CURSOR_MOTION_BUDGET_MS)?;
    loop {
        let elapsed = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        if elapsed >= motion.duration_ms() || budget.is_expired() {
            return draw_at(client, cursor, to);
        }
        draw_at(client, cursor, motion.sample(elapsed))?;
    }
}

/// Shows `cursor` at `point` and records it as the pose.
pub(crate) fn draw_at(
    client: &mut LayerClient,
    cursor: &mut AgentCursor,
    point: Point,
) -> Result<(), AdapterError> {
    draw_within(client, cursor, point, FRAME_WAIT_MS)
}

/// Like `draw_at`, giving the surface and its frame `wait_ms` to appear.
pub(crate) fn draw_within(
    client: &mut LayerClient,
    cursor: &mut AgentCursor,
    point: Point,
    wait_ms: u64,
) -> Result<(), AdapterError> {
    let placement = Placement::for_point(&point, cursor.bitmap.hotspot);
    cursor.pose = Some(point);
    let frame = Deadline::detached_after(wait_ms)?;
    client.draw(&cursor.bitmap, placement.margin, frame)
}
