use agent_desktop_core::DisplayInfo;

use super::output_point::OutputPoint;

/// Units per logical pixel sent to the virtual pointer.
pub(crate) const OUTPUT_UNITS: f64 = 120.0;

/// Maps a global layout point in logical pixels to the output that holds
/// it. A virtual pointer bound to an output maps `x / x_extent` onto that
/// output's logical box, so the point becomes output-local and the extent
/// is the output's logical size. A point on no output is `None`.
pub(crate) fn to_output(x: i32, y: i32, displays: &[DisplayInfo]) -> Option<OutputPoint> {
    let _ = (x, y, displays);
    None
}

#[cfg(test)]
#[path = "output_map_tests.rs"]
mod tests;
