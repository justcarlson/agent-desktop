//! The relative virtual pointer motions that carry the cursor to an aim, and
//! the check that it landed there before any button or wheel event.
//!
//! Hyprland forwards an absolute virtual pointer motion with no relative
//! motion and closes a frame after every motion, so XWayland moves its
//! absolute pointer device for it while it sends the wheel through its
//! relative one. The switch makes GDK3 reset its scroll baseline, and the
//! next notch scrolls nothing. A relative motion reaches XWayland with its
//! `wl_pointer.motion` in one frame, on the wheel's device
//! (probes/linux/FINDINGS.md L44-1).

use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode, Point};
use serde_json::json;

/// How far from the aim, in logical pixels on each axis, the cursor may
/// read and still count as on it.
const LANDING_TOLERANCE: f64 = 0.5;

/// The relative motions from `from` to `aim`, in logical pixels. A `first`
/// motion passes one pixel beside the aim, toward `beside_dx`, so Hyprland
/// hit-tests the aim even when the cursor already rests on it. A motion of
/// zero pixels is left out.
pub(crate) fn relative_moves(
    from: &Point,
    aim: (i32, i32),
    first: bool,
    beside_dx: i32,
) -> Vec<(f64, f64)> {
    let aim = (f64::from(aim.0), f64::from(aim.1));
    let stops = match first {
        true => vec![(aim.0 + f64::from(beside_dx), aim.1), aim],
        false => vec![aim],
    };
    let mut at = (from.x, from.y);
    let mut moves = Vec::new();
    for stop in stops {
        let delta = (stop.0 - at.0, stop.1 - at.1);
        if delta != (0.0, 0.0) {
            moves.push(delta);
        }
        at = stop;
    }
    moves
}

/// Whether `landed`, the cursor the compositor reports after the motions,
/// is on `aim`. A cursor off it refuses the run before any button or wheel
/// event.
pub(crate) fn confirm_landed(landed: &Point, aim: (i32, i32)) -> Result<(), AdapterError> {
    let on_aim = (landed.x - f64::from(aim.0)).abs() < LANDING_TOLERANCE
        && (landed.y - f64::from(aim.1)).abs() < LANDING_TOLERANCE;
    if on_aim {
        return Ok(());
    }
    Err(AdapterError::new(
        ErrorCode::ActionFailed,
        format!(
            "The cursor landed at ({}, {}), not on the aim ({}, {})",
            landed.x, landed.y, aim.0, aim.1
        ),
    )
    .with_details(json!({
        "kind": "hyprland_pointer",
        "cause": "cursor_missed_aim",
        "aim": {"x": aim.0, "y": aim.1},
        "landed": {"x": landed.x, "y": landed.y},
        "physical_delivery_started": false,
    }))
    .with_disposition(DeliverySemantics::not_delivered())
    .with_suggestion("Run 'snapshot' and retry once nothing else moves the cursor"))
}

/// Marks a failed cursor read as missing landing evidence: the run refuses
/// before any button or wheel event.
pub(crate) fn no_cursor_evidence(error: AdapterError) -> AdapterError {
    let mut details = match error.details.clone() {
        Some(serde_json::Value::Object(details)) => details,
        _ => serde_json::Map::new(),
    };
    details.insert("kind".to_owned(), json!("hyprland_pointer"));
    details.insert("cause".to_owned(), json!("cursor_unread"));
    details.insert("physical_delivery_started".to_owned(), json!(false));
    error
        .with_details(serde_json::Value::Object(details))
        .with_disposition(DeliverySemantics::not_delivered())
}

#[cfg(test)]
#[path = "relative_motion_tests.rs"]
mod tests;
