//! Where a cursor surface sits for a point. The surface is anchored to the
//! top-left of the output the compositor picks, so its margin is the point
//! less the arrow's hotspot, in that output's logical pixels. Output choice,
//! negative origins, fractional scales and hidden targets are not handled
//! here yet; every point maps to the compositor's chosen output.

use agent_desktop_core::Point;

use super::arrow_frame::ArrowFrame;
use super::output_layout::OutputLayout;

/// One surface placement: the output it is pinned to, if any, its top-left
/// margin and logical size on that output, its buffer size, the physical
/// offset the arrow is drawn at inside the buffer, and the output's scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Placement {
    pub(crate) output: Option<u32>,
    pub(crate) margin: (i32, i32),
    pub(crate) logical: (i32, i32),
    pub(crate) buffer: (i32, i32),
    pub(crate) offset: (i32, i32),
    pub(crate) scale: f64,
}

impl Placement {
    /// The placement that puts `hotspot` of the surface on `point`.
    pub(crate) fn for_point(point: &Point, hotspot: (f64, f64)) -> Self {
        let axis = |value: f64| {
            value
                .round()
                .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
        };
        Self {
            output: None,
            margin: (axis(point.x - hotspot.0), axis(point.y - hotspot.1)),
            logical: (0, 0),
            buffer: (0, 0),
            offset: (0, 0),
            scale: 1.0,
        }
    }
}

/// Every surface that shows the arrow with its tip on `point`.
pub(crate) fn place(
    point: &Point,
    outputs: &[OutputLayout],
    arrow_at: impl Fn(f64) -> ArrowFrame,
) -> Vec<Placement> {
    let _ = outputs;
    vec![Placement::for_point(point, arrow_at(1.0).hotspot)]
}

#[cfg(test)]
#[path = "placement_tests.rs"]
mod tests;
