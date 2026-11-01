//! One output the renderer has bound: its registry name, its `wl_output`
//! and `zxdg_output_v1` objects, and what their events have said so far.

use wayland_client::Proxy;
use wayland_client::protocol::wl_output;
use wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_v1;

use super::output_geometry::OutputGeometry;
use super::output_layout::OutputLayout;
use super::output_scale::OutputScale;

/// A bound output and its state.
#[derive(Debug)]
pub(crate) struct TrackedOutput {
    pub(crate) name: u32,
    pub(crate) output: wl_output::WlOutput,
    pub(crate) xdg: zxdg_output_v1::ZxdgOutputV1,
    pub(crate) geometry: OutputGeometry,
    pub(crate) scale: OutputScale,
}

impl TrackedOutput {
    /// The output in global layout logical pixels, once its origin and a
    /// positive size are known.
    pub(crate) fn layout(&self, fractional_manager: bool) -> Option<OutputLayout> {
        let (x, y) = self.geometry.origin?;
        let (width, height) = self.geometry.size.filter(|(w, h)| *w > 0 && *h > 0)?;
        Some(OutputLayout {
            output: self.name,
            origin: (f64::from(x), f64::from(y)),
            size: (f64::from(width), f64::from(height)),
            scale: self.scale.resolve(&self.geometry, fractional_manager),
        })
    }

    /// Releases the output's objects.
    pub(crate) fn release(&self) {
        self.xdg.destroy();
        if self.output.version() >= 3 {
            self.output.release();
        }
    }
}
