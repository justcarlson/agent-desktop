//! One cursor's overlay surface: a layer-shell surface on the overlay layer
//! under the `agent-desktop-cursor` namespace, anchored top-left, with an
//! empty input region, no keyboard interactivity and an exclusive zone of
//! -1, so it neither takes input nor moves other surfaces.

use agent_desktop_core::Deadline;
use wayland_client::protocol::wl_surface;
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

use super::frame_mark::FrameMark;
use super::layer_globals::LayerGlobals;
use super::layer_link::LayerLink;
use super::layer_state::LayerState;
use super::shm_canvas::ShmCanvas;
use crate::system::wayland::wayland_fault::WaylandFault;

/// The layer-shell namespace every agent cursor surface carries.
pub(crate) const CURSOR_NAMESPACE: &str = "agent-desktop-cursor";

/// A configured cursor surface and the buffers it shows.
pub(crate) struct CursorSurface {
    surface: wl_surface::WlSurface,
    layer: zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
    canvas: ShmCanvas,
    margin: (i32, i32),
}

impl CursorSurface {
    /// Creates the surface, commits it without a buffer and waits for its
    /// first configure.
    pub(crate) fn create(
        link: &mut LayerLink,
        state: &mut LayerState,
        globals: &LayerGlobals,
        canvas: ShmCanvas,
        deadline: Deadline,
    ) -> Result<Self, WaylandFault> {
        let handle = link.handle();
        let surface = globals.compositor.create_surface(&handle, ());
        let layer = globals.layer_shell.get_layer_surface(
            &surface,
            None,
            zwlr_layer_shell_v1::Layer::Overlay,
            CURSOR_NAMESPACE.to_owned(),
            &handle,
            (),
        );
        let (width, height) = canvas.size;
        layer.set_size(width as u32, height as u32);
        layer.set_anchor(zwlr_layer_surface_v1::Anchor::Top | zwlr_layer_surface_v1::Anchor::Left);
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::None);
        let region = globals.compositor.create_region(&handle, ());
        surface.set_input_region(Some(&region));
        region.destroy();
        state.configured = None;
        surface.commit();
        let made = Self {
            surface,
            layer,
            canvas,
            margin: (0, 0),
        };
        let configured = link.wait(
            state,
            |state| state.configured.is_some() || state.closed,
            deadline,
        );
        if let Err(fault) = configured {
            made.destroy();
            return Err(fault);
        }
        Ok(made)
    }

    /// Moves the surface so its top-left corner sits at `margin` and shows
    /// the drawn arrow, or the clear frame when `drawn` is false, then waits
    /// for the frame callback. A compositor that shows nothing, such as one
    /// with every output off, sends no callback; the wait then ends at
    /// `deadline` without an error.
    pub(crate) fn show(
        &mut self,
        link: &mut LayerLink,
        state: &mut LayerState,
        frame: (i32, i32, bool),
        deadline: Deadline,
    ) -> Result<(), WaylandFault> {
        let (left, top, drawn) = frame;
        if self.margin != (left, top) {
            self.layer.set_margin(top, 0, 0, left);
            self.margin = (left, top);
        }
        let buffer = if drawn {
            &self.canvas.drawn
        } else {
            &self.canvas.clear
        };
        self.surface.attach(Some(buffer), 0, 0);
        self.surface
            .damage(0, 0, self.canvas.size.0, self.canvas.size.1);
        state.frames_sent = state.frames_sent.wrapping_add(1);
        let mark = state.frames_sent;
        let _callback = self.surface.frame(&link.handle(), FrameMark(mark));
        self.surface.commit();
        match link.wait(
            state,
            |state| state.frames_done >= mark || state.closed,
            deadline,
        ) {
            Err(WaylandFault::Silent) => Ok(()),
            other => other,
        }
    }

    /// Where the surface's top-left corner sits.
    pub(crate) fn margin(&self) -> (i32, i32) {
        self.margin
    }

    /// Destroys the layer surface, the surface and the buffers.
    pub(crate) fn destroy(&self) {
        self.layer.destroy();
        self.surface.destroy();
        self.canvas.destroy();
    }
}
