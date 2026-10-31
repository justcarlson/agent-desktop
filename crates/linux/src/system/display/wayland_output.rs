use agent_desktop_core::{DisplayInfo, Rect};

/// Scales are kept in 120ths, the unit `wp_fractional_scale_v1` uses, so a
/// ratio of rounded integer sizes lands on the scale the compositor set.
const SCALE_STEP: f64 = 120.0;

/// One output as `wl_output` and `zxdg_output_v1` describe it: the connector
/// name, the logical position and size in the compositor's layout, the
/// current mode in pixels, whether its transform turns it a quarter, and the
/// integer scale `wl_output` reports.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WaylandOutput {
    pub(crate) name: Option<String>,
    pub(crate) position: Option<(i32, i32)>,
    pub(crate) logical: Option<(i32, i32)>,
    pub(crate) mode: Option<(i32, i32)>,
    pub(crate) turned: bool,
    pub(crate) integer_scale: i32,
}

impl WaylandOutput {
    /// The output's scale: its mode width, turned with the output, over its
    /// logical width. An output without a mode reports its integer scale.
    pub(crate) fn scale(&self) -> f64 {
        let fallback = f64::from(self.integer_scale.max(1));
        let (Some((mode_width, mode_height)), Some((logical_width, _))) = (self.mode, self.logical)
        else {
            return fallback;
        };
        let physical = if self.turned { mode_height } else { mode_width };
        if logical_width <= 0 || physical <= 0 {
            return fallback;
        }
        (f64::from(physical) / f64::from(logical_width) * SCALE_STEP).round() / SCALE_STEP
    }

    /// The display this output is, or `None` when the compositor left out
    /// its name, its position or a positive size.
    pub(crate) fn display(&self) -> Option<DisplayInfo> {
        let name = self.name.as_ref().filter(|name| !name.is_empty())?;
        let (x, y) = self.position?;
        let (width, height) = self.logical.filter(|(w, h)| *w > 0 && *h > 0)?;
        Some(DisplayInfo {
            id: name.clone(),
            bounds: Rect {
                x: f64::from(x),
                y: f64::from(y),
                width: f64::from(width),
                height: f64::from(height),
            },
            is_primary: false,
            scale: self.scale(),
        })
    }
}

#[cfg(test)]
#[path = "wayland_output_tests.rs"]
mod tests;
