//! The agent cursor's arrow, drawn on the CPU into premultiplied ARGB8888
//! pixels in the little-endian byte order `wl_shm` expects. The outline
//! matches the macOS overlay's arrow, scaled by the style's size.

use agent_desktop_core::CursorOverlayStyle;

const OUTLINE: [(f64, f64); 7] = [
    (0.0, 0.0),
    (0.0, 24.0),
    (6.0, 18.0),
    (11.0, 29.0),
    (16.0, 26.0),
    (11.0, 16.0),
    (21.0, 16.0),
];
const SUPERSAMPLE: u32 = 4;
const INSET: f64 = 1.0;
const RIM_WIDTH: f64 = 1.5;

/// A drawn arrow and the pixel its tip sits on.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ArrowBitmap {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) pixels: Vec<u8>,
    pub(crate) hotspot: (f64, f64),
}

impl ArrowBitmap {
    /// Draws the arrow in `style`'s fill and rim colors at its size.
    pub(crate) fn draw(style: &CursorOverlayStyle) -> Self {
        Self::draw_for_output(style, 1.0)
    }

    /// Draws the arrow for an output at `output_scale`: the style's size,
    /// kept within its bounds, times the output's scale.
    pub(crate) fn draw_for_output(style: &CursorOverlayStyle, output_scale: f64) -> Self {
        let scale = style.size().clamp(0.25, 8.0) * output_scale;
        let outline: Vec<(f64, f64)> = OUTLINE
            .iter()
            .map(|(x, y)| (x * scale + INSET, y * scale + INSET))
            .collect();
        let width = (22.0 * scale).ceil() as u32 + 2;
        let height = (30.0 * scale).ceil() as u32 + 2;
        let rim_width = RIM_WIDTH * scale.max(1.0);
        let colors = [style.rim_rgb(), style.fill_rgb()];
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height {
            for column in 0..width {
                let pixel = shade(&outline, (column, row), rim_width, colors);
                pixels.extend_from_slice(&pixel);
            }
        }
        Self {
            width,
            height,
            pixels,
            hotspot: (INSET, INSET),
        }
    }

    /// The bytes of one row.
    pub(crate) fn stride(&self) -> u32 {
        self.width * 4
    }
}

fn shade(outline: &[(f64, f64)], at: (u32, u32), rim_width: f64, colors: [[f64; 3]; 2]) -> [u8; 4] {
    let mut sums = [0.0_f64; 4];
    for sub_y in 0..SUPERSAMPLE {
        for sub_x in 0..SUPERSAMPLE {
            let x = f64::from(at.0) + (f64::from(sub_x) + 0.5) / f64::from(SUPERSAMPLE);
            let y = f64::from(at.1) + (f64::from(sub_y) + 0.5) / f64::from(SUPERSAMPLE);
            if !inside(outline, x, y) {
                continue;
            }
            let rim = edge_distance(outline, x, y) <= rim_width;
            let color = if rim { colors[0] } else { colors[1] };
            sums[0] += color[0];
            sums[1] += color[1];
            sums[2] += color[2];
            sums[3] += 1.0;
        }
    }
    let samples = f64::from(SUPERSAMPLE * SUPERSAMPLE);
    let channel = |sum: f64| (sum / samples * 255.0).round().clamp(0.0, 255.0) as u8;
    [
        channel(sums[2]),
        channel(sums[1]),
        channel(sums[0]),
        channel(sums[3]),
    ]
}

fn inside(outline: &[(f64, f64)], x: f64, y: f64) -> bool {
    let mut crossings = false;
    let mut previous = outline[outline.len() - 1];
    for &current in outline {
        let (ax, ay) = previous;
        let (bx, by) = current;
        if (ay > y) != (by > y) && x < (bx - ax) * (y - ay) / (by - ay) + ax {
            crossings = !crossings;
        }
        previous = current;
    }
    crossings
}

fn edge_distance(outline: &[(f64, f64)], x: f64, y: f64) -> f64 {
    let mut nearest = f64::INFINITY;
    let mut previous = outline[outline.len() - 1];
    for &current in outline {
        nearest = nearest.min(segment_distance(previous, current, (x, y)));
        previous = current;
    }
    nearest
}

fn segment_distance(a: (f64, f64), b: (f64, f64), p: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx * dx + dy * dy;
    let along = if length == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length).clamp(0.0, 1.0)
    };
    (p.0 - (a.0 + along * dx)).hypot(p.1 - (a.1 + along * dy))
}

#[cfg(test)]
#[path = "arrow_bitmap_tests.rs"]
mod tests;
