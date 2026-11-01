use agent_desktop_core::{DisplayInfo, Point};

use super::output_point::OutputPoint;
use crate::input::physical::shell_aim::display_under;

/// Units per logical pixel sent to the virtual pointer.
pub(crate) const OUTPUT_UNITS: f64 = 120.0;

/// Maps a global layout point in logical pixels to the output that holds
/// it. A virtual pointer bound to an output maps `x / x_extent` onto that
/// output's logical box, so the point becomes output-local and the extent
/// is the output's logical size. A point on no output is `None`.
pub(crate) fn to_output(x: i32, y: i32, displays: &[DisplayInfo]) -> Option<OutputPoint> {
    let point = Point {
        x: f64::from(x),
        y: f64::from(y),
    };
    let display = display_under(&point, displays)?;
    let bounds = &display.bounds;
    Some(OutputPoint {
        output: display.id.clone(),
        x: units(point.x - bounds.x)?,
        y: units(point.y - bounds.y)?,
        x_extent: units(bounds.width)?,
        y_extent: units(bounds.height)?,
    })
}

fn units(logical: f64) -> Option<u32> {
    let scaled = (logical * OUTPUT_UNITS).round();
    (scaled.is_finite() && scaled >= 0.0 && scaled <= f64::from(u32::MAX)).then_some(scaled as u32)
}

#[cfg(test)]
#[path = "output_map_tests.rs"]
mod tests;
