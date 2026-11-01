//! What a new cursor surface is built from: its buffers, the placement it
//! is sized for, and the output it is pinned to, if any.

use wayland_client::protocol::wl_output;

use super::placement::Placement;
use super::shm_canvas::ShmCanvas;

/// The parts of one cursor surface.
pub(crate) struct SurfaceParts {
    pub(crate) canvas: ShmCanvas,
    pub(crate) placement: Placement,
    pub(crate) output: Option<wl_output::WlOutput>,
}
