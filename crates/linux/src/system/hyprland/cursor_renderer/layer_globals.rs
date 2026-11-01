//! The globals the renderer binds. It binds only what drawing an overlay
//! needs, and never a seat or an activation global, so it can neither take
//! input nor ask for focus.

use agent_desktop_core::AdapterError;
use wayland_client::protocol::{wl_compositor, wl_shm};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1;

use super::layer_link::LayerLink;
use super::layer_state::LayerState;
use crate::system::hyprland::toplevel::bound_socket::unadvertised;

const COMPOSITOR: &str = "wl_compositor";
const SHM: &str = "wl_shm";
const LAYER_SHELL: &str = "zwlr_layer_shell_v1";
const OUTPUT: &str = "wl_output";

/// The bound globals a cursor surface is built from.
pub(crate) struct LayerGlobals {
    pub(crate) compositor: wl_compositor::WlCompositor,
    pub(crate) shm: wl_shm::WlShm,
    pub(crate) layer_shell: zwlr_layer_shell_v1::ZwlrLayerShellV1,
}

impl LayerGlobals {
    /// Binds the compositor, shared memory and layer shell globals `state`
    /// lists, or refuses when one is missing.
    pub(crate) fn bind(link: &LayerLink, state: &LayerState) -> Result<Self, AdapterError> {
        let need = "the agent cursor renderer";
        let find = |interface: &str| {
            state
                .exchange
                .find(interface, 1)
                .cloned()
                .ok_or_else(|| unadvertised(interface, need))
        };
        let compositor = find(COMPOSITOR)?;
        let shm = find(SHM)?;
        let layer_shell = find(LAYER_SHELL)?;
        Ok(Self {
            compositor: link.bind(&compositor, 4),
            shm: link.bind(&shm, 1),
            layer_shell: link.bind(&layer_shell, 4),
        })
    }
}

/// The number of outputs the compositor advertises.
pub(crate) fn output_count(state: &LayerState) -> usize {
    state.exchange.all(OUTPUT).count()
}
