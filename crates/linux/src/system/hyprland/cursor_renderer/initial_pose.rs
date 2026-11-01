//! Where a new renderer first shows the cursor: the middle of the focused
//! monitor, in that monitor's logical pixels. The cursor surface is placed
//! on the focused monitor, so this is a point on its own output. A reply
//! that names no usable monitor gives the origin.

use agent_desktop_core::Point;
use serde_json::Value;

/// The initial cursor point from a Hyprland `j/monitors` reply.
pub(crate) fn initial_point(monitors: &[u8]) -> Point {
    let origin = Point { x: 0.0, y: 0.0 };
    let Ok(Value::Array(monitors)) = serde_json::from_slice::<Value>(monitors) else {
        return origin;
    };
    let chosen = monitors
        .iter()
        .find(|monitor| monitor["focused"].as_bool() == Some(true))
        .or_else(|| monitors.first());
    chosen.and_then(middle_of).unwrap_or(origin)
}

fn middle_of(monitor: &Value) -> Option<Point> {
    let width = monitor["width"].as_f64()?;
    let height = monitor["height"].as_f64()?;
    let scale = monitor["scale"].as_f64().filter(|scale| *scale > 0.0)?;
    let turned = monitor["transform"]
        .as_u64()
        .is_some_and(|turn| turn % 2 == 1);
    let (across, down) = if turned {
        (height, width)
    } else {
        (width, height)
    };
    let middle = Point {
        x: (across / scale / 2.0).round(),
        y: (down / scale / 2.0).round(),
    };
    (middle.x > 0.0 && middle.y > 0.0).then_some(middle)
}

#[cfg(test)]
#[path = "initial_pose_tests.rs"]
mod tests;
