//! Where a new renderer first shows the cursor: the middle of the focused
//! monitor, in global layout logical pixels, the monitor's origin plus half
//! its logical size. Cursor surfaces are placed from global points, so this
//! point lands on the focused monitor wherever it sits in the layout. A
//! reply that names no usable monitor gives the origin.

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
    let half = ((across / scale / 2.0).round(), (down / scale / 2.0).round());
    let origin = (
        monitor["x"].as_f64().unwrap_or(0.0),
        monitor["y"].as_f64().unwrap_or(0.0),
    );
    (half.0 > 0.0 && half.1 > 0.0).then_some(Point {
        x: origin.0 + half.0,
        y: origin.1 + half.1,
    })
}

#[cfg(test)]
#[path = "initial_pose_tests.rs"]
mod tests;
