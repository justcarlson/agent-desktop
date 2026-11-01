//! One cursor's overlay surface: a layer-shell surface on the overlay layer
//! under the `agent-desktop-cursor` namespace, anchored top-left, with an
//! empty input region, no keyboard interactivity and an exclusive zone of
//! -1, so it neither takes input nor moves other surfaces. A surface pinned
//! to an output has that output's logical size as its viewport destination,
//! so its buffer draws at the output's scale.

use agent_desktop_core::Deadline;
use wayland_client::Proxy;
use wayland_client::backend::ObjectId;
use wayland_client::protocol::wl_surface;
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1;
use wayland_protocols::wp::viewporter::client::wp_viewport;
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

use super::frame_mark::FrameMark;
use super::layer_globals::LayerGlobals;
use super::layer_link::LayerLink;
use super::layer_state::LayerState;
use super::placement::Placement;
use super::shm_canvas::ShmCanvas;
use super::surface_parts::SurfaceParts;
use crate::system::wayland::wayland_fault::WaylandFault;

/// The layer-shell namespace every agent cursor surface carries.
pub(crate) const CURSOR_NAMESPACE: &str = "agent-desktop-cursor";

/// A configured cursor surface and the buffers it shows.
pub(crate) struct CursorSurface {
    surface: wl_surface::WlSurface,
    layer: zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
    canvas: ShmCanvas,
    margin: (i32, i32),
    logical: (i32, i32),
    viewport: Option<wp_viewport::WpViewport>,
    fractional: Option<wp_fractional_scale_v1::WpFractionalScaleV1>,
}

impl CursorSurface {
    /// Creates the surface, pinned to `parts`' output when it names one,
    /// commits it without a buffer and waits for its first configure.
    pub(crate) fn create(
        link: &mut LayerLink,
        state: &mut LayerState,
        globals: &LayerGlobals,
        parts: SurfaceParts,
        deadline: Deadline,
    ) -> Result<Self, WaylandFault> {
        let handle = link.handle();
        let pinned = parts.placement.output.filter(|_| parts.output.is_some());
        let surface = globals.compositor.create_surface(&handle, pinned);
        let layer = globals.layer_shell.get_layer_surface(
            &surface,
            parts.output.as_ref(),
            zwlr_layer_shell_v1::Layer::Overlay,
            CURSOR_NAMESPACE.to_owned(),
            &handle,
            pinned,
        );
        let logical = parts.placement.logical;
        let viewport = globals.viewporter.as_ref().map(|viewporter| {
            let viewport = viewporter.get_viewport(&surface, &handle, ());
            viewport.set_destination(logical.0, logical.1);
            viewport
        });
        let fractional = globals
            .fractional
            .as_ref()
            .zip(pinned)
            .map(|(manager, name)| manager.get_fractional_scale(&surface, &handle, name));
        layer.set_size(logical.0.max(1) as u32, logical.1.max(1) as u32);
        layer.set_anchor(zwlr_layer_surface_v1::Anchor::Top | zwlr_layer_surface_v1::Anchor::Left);
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::None);
        let region = globals.compositor.create_region(&handle, ());
        surface.set_input_region(Some(&region));
        region.destroy();
        state.configured = None;
        surface.commit();
        let layer_id = layer.id();
        let made = Self {
            surface,
            layer,
            canvas: parts.canvas,
            margin: (0, 0),
            logical,
            viewport,
            fractional,
        };
        let configured = link.wait(
            state,
            |state| {
                state.configured.is_some()
                    || state.closed
                    || state.outputs.closed_layers.contains(&layer_id)
            },
            deadline,
        );
        if let Err(fault) = configured {
            made.destroy(state);
            return Err(fault);
        }
        Ok(made)
    }

    /// Whether this surface has the sizes `placement` needs.
    pub(crate) fn fits(&self, placement: &Placement) -> bool {
        self.logical == placement.logical && self.canvas.size == placement.buffer
    }

    /// The layer surface's object id, which a `closed` event names.
    pub(crate) fn layer_id(&self) -> ObjectId {
        self.layer.id()
    }

    /// Moves the surface so its top-left corner sits at `frame`'s margin and
    /// commits the arrow at `frame`'s offset, or the clear frame when it
    /// has none, without waiting. It returns the frame mark the callback
    /// reaches, so several cursors can share one compositor frame.
    pub(crate) fn commit_frame(
        &mut self,
        link: &LayerLink,
        state: &mut LayerState,
        frame: ((i32, i32), Option<(i32, i32)>),
    ) -> u32 {
        let ((left, top), offset) = frame;
        if self.margin != (left, top) {
            self.layer.set_margin(top, 0, 0, left);
            self.margin = (left, top);
        }
        self.surface.attach(Some(self.canvas.frame(offset)), 0, 0);
        self.surface.damage(0, 0, self.logical.0, self.logical.1);
        state.frames_sent = state.frames_sent.wrapping_add(1);
        let mark = state.frames_sent;
        let _callback = self.surface.frame(&link.handle(), FrameMark(mark));
        state.frames_pending.insert(mark, self.surface.id());
        self.surface.commit();
        mark
    }

    /// Where the surface's top-left corner sits.
    pub(crate) fn margin(&self) -> (i32, i32) {
        self.margin
    }

    /// Destroys the layer surface, the surface and the buffers, and drops
    /// the frames still pending on it, whose callbacks never come.
    pub(crate) fn destroy(&self, state: &mut LayerState) {
        let surface = self.surface.id();
        state
            .frames_pending
            .retain(|_, pending| *pending != surface);
        if let Some(fractional) = &self.fractional {
            fractional.destroy();
        }
        if let Some(viewport) = &self.viewport {
            viewport.destroy();
        }
        self.layer.destroy();
        self.surface.destroy();
        self.canvas.destroy();
    }
}

/// Waits until the callback of every frame `frames` spans has fired: the
/// marks after its first value up to and including its second, on every
/// surface still alive. A compositor that shows nothing sends no callback,
/// so the deadline passing is not an error.
pub(crate) fn settle(
    link: &mut LayerLink,
    state: &mut LayerState,
    frames: (u32, u32),
    deadline: Deadline,
) -> Result<(), WaylandFault> {
    match link.wait(
        state,
        |state| state.closed || !any_pending(state, frames),
        deadline,
    ) {
        Err(WaylandFault::Silent) => Ok(()),
        other => other,
    }
}

fn any_pending(state: &LayerState, (after, through): (u32, u32)) -> bool {
    let span = through.wrapping_sub(after);
    state
        .frames_pending
        .keys()
        .any(|mark| mark.wrapping_sub(after).wrapping_sub(1) < span)
}
