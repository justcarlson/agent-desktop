//! The scale a cursor surface on one output draws at. The compositor's
//! `wp_fractional_scale_v1` preference wins; without a fractional scale
//! manager the surface's preferred buffer scale comes next; otherwise the
//! output's mode over its logical size, then its integer `wl_output` scale.

use super::output_geometry::OutputGeometry;

const SCALE_STEP: f64 = 120.0;

/// The scale hints one output's surfaces and `wl_output` have sent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct OutputScale {
    pub(crate) fractional: Option<u32>,
    pub(crate) buffer: Option<i32>,
    pub(crate) integer: Option<i32>,
}

impl OutputScale {
    /// The scale to draw at; `fractional_manager` says whether the
    /// compositor offers fractional scale at all.
    pub(crate) fn resolve(&self, geometry: &OutputGeometry, fractional_manager: bool) -> f64 {
        if let Some(steps) = self.fractional.filter(|steps| *steps > 0) {
            return f64::from(steps) / SCALE_STEP;
        }
        if !fractional_manager && let Some(factor) = self.buffer.filter(|factor| *factor > 0) {
            return f64::from(factor);
        }
        geometry
            .mode_scale()
            .unwrap_or_else(|| f64::from(self.integer.unwrap_or(1).max(1)))
    }
}

#[cfg(test)]
#[path = "output_scale_tests.rs"]
mod tests;
