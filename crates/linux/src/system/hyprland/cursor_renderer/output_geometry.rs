//! Where one output sits as `wl_output` and `zxdg_output_v1` describe it:
//! its logical origin and size in the global layout, its current mode in
//! physical pixels, and whether its transform turns it a quarter.

const SCALE_STEP: f64 = 120.0;

/// One output's layout geometry, filled in as its events arrive.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct OutputGeometry {
    pub(crate) origin: Option<(i32, i32)>,
    pub(crate) size: Option<(i32, i32)>,
    pub(crate) mode: Option<(i32, i32)>,
    pub(crate) turned: bool,
}

impl OutputGeometry {
    /// The mode width, turned with the output, over the logical width, in
    /// 120ths; `None` until both are known and positive.
    pub(crate) fn mode_scale(&self) -> Option<f64> {
        let (mode_width, mode_height) = self.mode?;
        let (logical_width, _) = self.size?;
        let physical = if self.turned { mode_height } else { mode_width };
        if logical_width <= 0 || physical <= 0 {
            return None;
        }
        Some((f64::from(physical) / f64::from(logical_width) * SCALE_STEP).round() / SCALE_STEP)
    }
}
