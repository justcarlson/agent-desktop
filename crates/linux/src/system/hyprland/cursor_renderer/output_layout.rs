//! One output as the renderer places cursors on it: the registry name of
//! its `wl_output`, its logical origin and size in the global layout, and
//! the scale its surfaces draw at.

use agent_desktop_core::Point;

/// One output in global layout logical pixels.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OutputLayout {
    pub(crate) output: u32,
    pub(crate) origin: (f64, f64),
    pub(crate) size: (f64, f64),
    pub(crate) scale: f64,
}

impl OutputLayout {
    /// Whether `point` lies on this output.
    pub(crate) fn contains(&self, point: &Point) -> bool {
        let (x, y) = self.origin;
        let (width, height) = self.size;
        point.x >= x && point.x < x + width && point.y >= y && point.y < y + height
    }
}
