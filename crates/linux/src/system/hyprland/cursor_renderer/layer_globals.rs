//! The globals the renderer binds. It binds only what drawing an overlay
//! needs, and never a seat or an activation global, so it can neither take
//! input nor ask for focus. With `zxdg_output_manager_v1` and `wp_viewporter`
//! it also binds the outputs, so it can pin a surface to each output at that
//! output's scale; `wp_fractional_scale_v1` is used when offered.

use agent_desktop_core::AdapterError;
use wayland_client::protocol::{wl_compositor, wl_shm};
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1;
use wayland_protocols::wp::viewporter::client::wp_viewporter;
use wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_manager_v1;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1;

use super::layer_link::LayerLink;
use super::layer_state::LayerState;
use crate::system::hyprland::toplevel::bound_socket::unadvertised;

const COMPOSITOR: &str = "wl_compositor";
const SHM: &str = "wl_shm";
const LAYER_SHELL: &str = "zwlr_layer_shell_v1";
const XDG_OUTPUTS: &str = "zxdg_output_manager_v1";
const VIEWPORTER: &str = "wp_viewporter";
const FRACTIONAL: &str = "wp_fractional_scale_manager_v1";
pub(crate) const OUTPUT: &str = "wl_output";
pub(crate) const OUTPUT_VERSION: u32 = 4;

/// The bound globals a cursor surface is built from.
pub(crate) struct LayerGlobals {
    pub(crate) compositor: wl_compositor::WlCompositor,
    pub(crate) shm: wl_shm::WlShm,
    pub(crate) layer_shell: zwlr_layer_shell_v1::ZwlrLayerShellV1,
    pub(crate) xdg_outputs: Option<zxdg_output_manager_v1::ZxdgOutputManagerV1>,
    pub(crate) viewporter: Option<wp_viewporter::WpViewporter>,
    pub(crate) fractional: Option<wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1>,
}

impl LayerGlobals {
    /// Binds the compositor, shared memory and layer shell globals `state`
    /// lists, or refuses when one is missing, and the layout globals when
    /// both are offered.
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
        let offered = |interface: &str| state.exchange.find(interface, 1).cloned();
        let layout = offered(XDG_OUTPUTS).zip(offered(VIEWPORTER));
        let fractional = layout.as_ref().and(offered(FRACTIONAL));
        Ok(Self {
            compositor: link.bind(&compositor, 6),
            shm: link.bind(&shm, 1),
            layer_shell: link.bind(&layer_shell, 4),
            xdg_outputs: layout.as_ref().map(|(xdg, _)| link.bind(xdg, 3)),
            viewporter: layout
                .as_ref()
                .map(|(_, viewporter)| link.bind(viewporter, 1)),
            fractional: fractional.map(|fractional| link.bind(&fractional, 1)),
        })
    }

    /// Whether the renderer can pin surfaces to outputs.
    pub(crate) fn lays_out(&self) -> bool {
        self.xdg_outputs.is_some() && self.viewporter.is_some()
    }
}

/// The number of outputs the compositor advertises.
pub(crate) fn output_count(state: &LayerState) -> usize {
    state.exchange.all(OUTPUT).count()
}
