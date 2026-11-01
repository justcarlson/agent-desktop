//! The renderer's Wayland client state: the registry, round trips, the layer
//! surface's configure and close events, and frame callbacks. Frame marks
//! count for the whole connection, so a rebuilt surface never waits on a
//! mark an earlier surface already reached. A closed surface pinned to an
//! output is noted for the client to rebuild, since its output went; a
//! closed unpinned surface closes the renderer.

use wayland_client::protocol::{
    wl_buffer, wl_callback, wl_compositor, wl_region, wl_registry, wl_shm, wl_shm_pool,
};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, delegate_dispatch, delegate_noop};
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1;
use wayland_protocols::wp::viewporter::client::{wp_viewport, wp_viewporter};
use wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_manager_v1;
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

use super::frame_mark::FrameMark;
use super::layer_outputs::LayerOutputs;
use crate::system::wayland::exchange::{Exchange, ExchangeHandler, HoldsExchange};

/// What the compositor has told the renderer so far.
#[derive(Debug, Default)]
pub(crate) struct LayerState {
    pub(crate) exchange: Exchange,
    pub(crate) configured: Option<u32>,
    pub(crate) closed: bool,
    pub(crate) frames_sent: u32,
    pub(crate) frames_done: u32,
    pub(crate) outputs: LayerOutputs,
}

impl HoldsExchange for LayerState {
    fn exchange(&self) -> &Exchange {
        &self.exchange
    }

    fn exchange_mut(&mut self) -> &mut Exchange {
        &mut self.exchange
    }
}

delegate_dispatch!(LayerState: [wl_registry::WlRegistry: ()] => ExchangeHandler);
delegate_dispatch!(LayerState: [wl_callback::WlCallback: u32] => ExchangeHandler);
delegate_noop!(LayerState: wl_compositor::WlCompositor);
delegate_noop!(LayerState: wl_region::WlRegion);
delegate_noop!(LayerState: wl_shm_pool::WlShmPool);
delegate_noop!(LayerState: zwlr_layer_shell_v1::ZwlrLayerShellV1);
delegate_noop!(LayerState: ignore wl_shm::WlShm);
delegate_noop!(LayerState: ignore wl_buffer::WlBuffer);
delegate_noop!(LayerState: zxdg_output_manager_v1::ZxdgOutputManagerV1);
delegate_noop!(LayerState: wp_viewporter::WpViewporter);
delegate_noop!(LayerState: wp_viewport::WpViewport);
delegate_noop!(LayerState: wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1);

impl Dispatch<wl_callback::WlCallback, FrameMark> for LayerState {
    fn event(
        state: &mut Self,
        _: &wl_callback::WlCallback,
        event: wl_callback::Event,
        mark: &FrameMark,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = event {
            state.frames_done = state.frames_done.max(mark.0);
        }
    }
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, Option<u32>> for LayerState {
    fn event(
        state: &mut Self,
        layer: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        pinned: &Option<u32>,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure { serial, .. } => {
                layer.ack_configure(serial);
                state.configured = Some(serial);
            }
            zwlr_layer_surface_v1::Event::Closed if pinned.is_some() => {
                state.outputs.closed_layers.push(layer.id());
                state.outputs.changed = true;
            }
            zwlr_layer_surface_v1::Event::Closed => state.closed = true,
            _ => {}
        }
    }
}
