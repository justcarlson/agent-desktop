//! The displays of a bound Hyprland instance, read from its IPC socket with
//! `j/monitors`. Hyprland places monitors in a logical layout: `x` and `y`
//! are layout coordinates, and `width` and `height` are the mode in pixels,
//! so a monitor's logical size is its mode turned by its transform and
//! divided by its scale. Hyprland has no primary monitor; the monitor with
//! the lowest id, the first one the compositor configured, is reported as
//! primary so the answer is stable while focus moves.

use agent_desktop_core::{AdapterError, Deadline, DisplayInfo, Rect};
use serde_json::Value;

use super::bound_request::bound_request;
use super::hyprland_binding::HyprlandBinding;

const MONITORS_COMMAND: &str = "j/monitors";

/// The bound instance's displays, primary first.
pub(crate) fn list(
    binding: &HyprlandBinding,
    deadline: Deadline,
) -> Result<Vec<DisplayInfo>, AdapterError> {
    let body = bound_request(binding, MONITORS_COMMAND, deadline)?;
    parse(&body).ok_or_else(|| {
        AdapterError::internal("The Hyprland monitor list could not be read")
            .with_suggestion("Retry once the compositor answers")
    })
}

/// Parses a `j/monitors` reply. A reply that is not a list of monitors with
/// the measured fields is `None`.
pub(crate) fn parse(body: &[u8]) -> Option<Vec<DisplayInfo>> {
    let monitors: Vec<Value> = serde_json::from_slice(body).ok()?;
    let mut read = monitors
        .iter()
        .map(|monitor| Some((monitor.get("id")?.as_i64()?, display_of(monitor)?)))
        .collect::<Option<Vec<_>>>()?;
    read.sort_by_key(|(id, _)| *id);
    let mut displays: Vec<DisplayInfo> = read.into_iter().map(|(_, display)| display).collect();
    if let Some(first) = displays.first_mut() {
        first.is_primary = true;
    }
    Some(displays)
}

fn display_of(monitor: &Value) -> Option<DisplayInfo> {
    let number = |key: &str| monitor.get(key).and_then(Value::as_f64);
    let scale = number("scale").filter(|scale| *scale > 0.0)?;
    let (mut width, mut height) = (number("width")?, number("height")?);
    let transform = monitor
        .get("transform")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if transform % 2 == 1 {
        std::mem::swap(&mut width, &mut height);
    }
    Some(DisplayInfo {
        id: monitor.get("name")?.as_str()?.to_owned(),
        bounds: Rect {
            x: number("x")?,
            y: number("y")?,
            width: width / scale,
            height: height / scale,
        },
        is_primary: false,
        scale,
    })
}

#[cfg(test)]
#[path = "monitors_tests.rs"]
mod tests;
