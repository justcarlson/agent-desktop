//! Where a cursor surface sits for a point. The surface is anchored to the
//! top-left of the output the compositor picks, so its margin is the point
//! less the arrow's hotspot, in that output's logical pixels. Output choice,
//! negative origins, fractional scales and hidden targets are not handled
//! here yet; every point maps to the compositor's chosen output.

use agent_desktop_core::Point;

/// One surface placement: the output it is pinned to, if any, and its
/// top-left margin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Placement {
    pub(crate) output: Option<u32>,
    pub(crate) margin: (i32, i32),
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
        }
    }
}

#[cfg(test)]
#[path = "placement_tests.rs"]
mod tests;
