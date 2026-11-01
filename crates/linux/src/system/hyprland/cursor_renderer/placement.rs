//! Where cursor surfaces sit for a point, in global layout logical pixels.
//! Each surface is pinned to one output and anchored to its top-left, so its
//! margin is `round(box.origin - output.origin)`, computed in `f64`; a
//! negative origin needs no special case. The box is the arrow's logical box
//! with its hotspot on the point, widened half a logical pixel up and left
//! so the hotspot's physical remainder is never negative.
//!
//! Each surface draws at its own output's scale: the arrow is drawn in
//! physical pixels, the buffer is `ceil(logical size × scale)`, the viewport
//! destination is the logical size, and the arrow sits at the whole
//! physical offset nearest the hotspot's remainder, so the hotspot lands
//! within one physical pixel of the point. A box that crosses an output
//! edge gets one surface on each output it touches. A point on no output
//! places nothing. With no output layout known, one unpinned surface the
//! size of the arrow draws at scale 1, its margin the point less the
//! hotspot, rounded.

use agent_desktop_core::Point;

use super::arrow_frame::ArrowFrame;
use super::output_layout::OutputLayout;

const SCALE_STEP: i64 = 120;
const WHOLE_DENOMINATOR_MAX: i64 = 8;
const SCALE_SLACK: f64 = 1e-9;

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

/// Every surface that shows the arrow with its tip on `point`. `arrow_at`
/// gives the arrow's physical frame at an output scale.
pub(crate) fn place(
    point: &Point,
    outputs: &[OutputLayout],
    mut arrow_at: impl FnMut(f64) -> ArrowFrame,
) -> Vec<Placement> {
    if outputs.is_empty() {
        return vec![unpinned(point, arrow_at(1.0))];
    }
    if !outputs.iter().any(|output| output.contains(point)) {
        return Vec::new();
    }
    outputs
        .iter()
        .map(|output| (output, on_output(point, output, arrow_at(output.scale))))
        .filter(|(output, placed)| touches(output, placed))
        .map(|(_, placed)| placed)
        .collect()
}

/// Whether `point` lies on some output, or no output layout is known.
pub(crate) fn on_some_output(point: &Point, outputs: &[OutputLayout]) -> bool {
    outputs.is_empty() || outputs.iter().any(|output| output.contains(point))
}

fn unpinned(point: &Point, frame: ArrowFrame) -> Placement {
    let size = (
        whole(f64::from(frame.size.0)),
        whole(f64::from(frame.size.1)),
    );
    Placement {
        output: None,
        margin: (
            whole((point.x - frame.hotspot.0).round()),
            whole((point.y - frame.hotspot.1).round()),
        ),
        logical: size,
        buffer: size,
        offset: (0, 0),
        scale: 1.0,
    }
}

fn on_output(point: &Point, output: &OutputLayout, frame: ArrowFrame) -> Placement {
    let scale = output.scale;
    let x = axis(
        point.x - output.origin.0,
        scale,
        frame.hotspot.0,
        frame.size.0,
    );
    let y = axis(
        point.y - output.origin.1,
        scale,
        frame.hotspot.1,
        frame.size.1,
    );
    Placement {
        output: Some(output.output),
        margin: (x.margin, y.margin),
        logical: (x.logical, y.logical),
        buffer: (x.buffer, y.buffer),
        offset: (x.offset, y.offset),
        scale,
    }
}

struct Axis {
    margin: i32,
    logical: i32,
    buffer: i32,
    offset: i32,
}

/// The largest whole physical offset the arrow is drawn at inside its
/// buffer on an output at `scale`.
pub(crate) fn reach(scale: f64) -> i32 {
    whole(scale.ceil()).max(1)
}

fn axis(local: f64, scale: f64, hotspot: f64, arrow: u32) -> Axis {
    let reach = reach(scale);
    let margin = whole((local - hotspot / scale - 0.5).round());
    let surface_origin = (f64::from(margin) * scale).round();
    let offset = whole((local * scale - hotspot - surface_origin).round()).clamp(0, reach);
    let needed = f64::from(arrow) + f64::from(reach);
    let step = whole_step(scale);
    let logical = whole((needed / scale / f64::from(step)).ceil()) * step;
    let buffer = whole((f64::from(logical) * scale - SCALE_SLACK).ceil());
    Axis {
        margin,
        logical,
        buffer,
        offset,
    }
}

/// The smallest logical length whose physical length at `scale` is whole,
/// when that length is small; 1 otherwise.
fn whole_step(scale: f64) -> i32 {
    let steps = (scale * SCALE_STEP as f64).round() as i64;
    if steps <= 0 {
        return 1;
    }
    let denominator = SCALE_STEP / gcd(steps, SCALE_STEP);
    if denominator <= WHOLE_DENOMINATOR_MAX {
        denominator as i32
    } else {
        1
    }
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

fn touches(output: &OutputLayout, placed: &Placement) -> bool {
    let left = output.origin.0 + f64::from(placed.margin.0);
    let top = output.origin.1 + f64::from(placed.margin.1);
    let right = left + f64::from(placed.logical.0);
    let bottom = top + f64::from(placed.logical.1);
    left < output.origin.0 + output.size.0
        && right > output.origin.0
        && top < output.origin.1 + output.size.1
        && bottom > output.origin.1
}

fn whole(value: f64) -> i32 {
    value.clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
}

#[cfg(test)]
#[path = "placement_tests.rs"]
mod tests;
