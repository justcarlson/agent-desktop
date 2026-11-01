//! The renderer's drawing side: one connection to the bound compositor, the
//! globals it binds, and one surface per cursor key, each created on its
//! first draw. Every agent cursor draws on its own surfaces, so one agent's
//! style or position never rebuilds or moves another's. When the compositor
//! offers the output layout, a cursor gets one surface per output its arrow
//! touches, each pinned to that output and drawn at its scale; outputs that
//! come or go are bound or released on the next pump.

use std::collections::BTreeMap;
use std::os::fd::BorrowedFd;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use agent_desktop_core::{AdapterError, Deadline, Point};
use wayland_client::protocol::wl_output;

use super::agent_cursor::AgentCursor;
use super::arrow_bitmap::ArrowBitmap;
use super::arrow_frame::ArrowFrame;
use super::cursor_surface::{CursorSurface, settle};
use super::layer_globals::{LayerGlobals, OUTPUT, OUTPUT_VERSION, output_count};
use super::layer_link::LayerLink;
use super::layer_state::LayerState;
use super::output_layout::OutputLayout;
use super::placement::{Placement, on_some_output, place};
use super::scaled_arrows::ScaledArrows;
use super::shm_canvas::ShmCanvas;
use super::surface_key::SurfaceKey;
use super::surface_parts::SurfaceParts;
use super::tracked_output::TrackedOutput;
use crate::system::wayland::global::Global;
use crate::system::wayland::wayland_fault::WaylandFault;

/// A connection that draws the session's cursors.
pub(crate) struct LayerClient {
    link: LayerLink,
    state: LayerState,
    globals: LayerGlobals,
    surfaces: BTreeMap<SurfaceKey, (CursorSurface, ArrowBitmap)>,
    socket: PathBuf,
    arrows: ScaledArrows,
}

impl LayerClient {
    /// Reads the registry over `stream`, a connection to the compositor at
    /// `socket`, binds the drawing globals and the outputs, and reads the
    /// output layout. No surface exists yet.
    pub(crate) fn over(
        stream: UnixStream,
        socket: &Path,
        deadline: Deadline,
    ) -> Result<Self, AdapterError> {
        let mut state = LayerState::default();
        let link = LayerLink::over(stream, &mut state, deadline)
            .map_err(|fault| fault.into_error(socket))?;
        let globals = LayerGlobals::bind(&link, &state)?;
        let mut client = Self {
            link,
            state,
            globals,
            surfaces: BTreeMap::new(),
            socket: socket.to_path_buf(),
            arrows: ScaledArrows::default(),
        };
        client.sync_outputs();
        client.roundtrip(deadline)?;
        client.state.outputs.changed = false;
        Ok(client)
    }

    /// Shows `bitmap` on the surface `key` with its top-left corner at
    /// `at`, and waits for the frame.
    #[cfg(test)]
    pub(crate) fn draw(
        &mut self,
        key: &SurfaceKey,
        bitmap: &ArrowBitmap,
        at: (i32, i32),
        deadline: Deadline,
    ) -> Result<(), AdapterError> {
        let size = (bitmap.width as i32, bitmap.height as i32);
        let placement = Placement {
            output: None,
            margin: at,
            logical: size,
            buffer: size,
            offset: (0, 0),
            scale: 1.0,
        };
        let after = self.state.frames_sent;
        let mark = self.stage(key, bitmap, &placement, deadline)?;
        self.settle((after, mark), deadline)
    }

    /// Commits `cursor`'s arrow with its tip on `point` on every output the
    /// arrow touches, clears that cursor's other surfaces, and returns the
    /// span of frames to settle on. A point on no output shows nothing.
    pub(crate) fn stage_point(
        &mut self,
        key: &str,
        cursor: &AgentCursor,
        point: &Point,
        deadline: Deadline,
    ) -> Result<(u32, u32), AdapterError> {
        let layout = self.layout();
        let arrows = &mut self.arrows;
        let placements = place(point, &layout, |scale| {
            ArrowFrame::of(&arrows.at(cursor, scale))
        });
        let after = self.state.frames_sent;
        let mut shown = Vec::new();
        for placement in &placements {
            if placement
                .output
                .is_some_and(|name| self.state.outputs.proxy(name).is_none())
            {
                continue;
            }
            let surface = SurfaceKey::placed(key, placement);
            let bitmap = self.arrows.at(cursor, placement.scale);
            self.stage(&surface, &bitmap, placement, deadline)?;
            shown.push(surface);
        }
        self.clear_frames(|surface| surface.cursor == key && !shown.contains(surface));
        Ok((after, self.state.frames_sent))
    }

    /// Whether `point` lies on an output, or the output layout is unknown.
    pub(crate) fn on_output(&self, point: &Point) -> bool {
        on_some_output(point, &self.layout())
    }

    /// Whether the output layout changed since the last call.
    pub(crate) fn take_output_change(&mut self) -> bool {
        std::mem::take(&mut self.state.outputs.changed)
    }

    fn layout(&self) -> Vec<OutputLayout> {
        if !self.globals.lays_out() {
            return Vec::new();
        }
        self.state.outputs.layout(self.globals.fractional.is_some())
    }

    /// Commits `bitmap` on the surface `key` as `placement` says, without
    /// waiting for the frame, creating or rebuilding that surface when its
    /// bitmap or sizes changed, and returns the frame mark to settle on.
    pub(crate) fn stage(
        &mut self,
        key: &SurfaceKey,
        bitmap: &ArrowBitmap,
        placement: &Placement,
        deadline: Deadline,
    ) -> Result<u32, AdapterError> {
        let current = self
            .surfaces
            .get(key)
            .is_some_and(|(surface, drawn)| drawn == bitmap && surface.fits(placement));
        if !current {
            self.drop_surface(key);
            let canvas = ShmCanvas::create(&self.globals, bitmap, placement, &self.link.handle())?;
            let output = placement
                .output
                .and_then(|name| self.state.outputs.proxy(name).cloned());
            let parts = SurfaceParts {
                canvas,
                placement: *placement,
                output,
            };
            let surface = CursorSurface::create(
                &mut self.link,
                &mut self.state,
                &self.globals,
                parts,
                deadline,
            )
            .map_err(|fault| self.error(fault))?;
            self.surfaces.insert(key.clone(), (surface, bitmap.clone()));
        }
        let Some((surface, _)) = self.surfaces.get_mut(key) else {
            return Ok(self.state.frames_sent);
        };
        let frame = (placement.margin, Some(placement.offset));
        Ok(surface.commit_frame(&self.link, &mut self.state, frame))
    }

    /// Waits until every frame in `frames` has shown on its own output, or
    /// the compositor shows nothing until `deadline`.
    pub(crate) fn settle(
        &mut self,
        frames: (u32, u32),
        deadline: Deadline,
    ) -> Result<(), AdapterError> {
        settle(&mut self.link, &mut self.state, frames, deadline).map_err(|fault| self.error(fault))
    }

    /// Commits the clear frame on every surface of `cursor`, where each
    /// sits, so that cursor shows nothing; other cursors are untouched.
    pub(crate) fn clear(&mut self, cursor: &str, deadline: Deadline) -> Result<(), AdapterError> {
        self.clear_where(|key| key.cursor == cursor, deadline)
    }

    /// Commits the clear frame on every surface, so no cursor shows.
    pub(crate) fn clear_all(&mut self, deadline: Deadline) -> Result<(), AdapterError> {
        self.clear_where(|_| true, deadline)
    }

    fn clear_where(
        &mut self,
        chosen: impl Fn(&SurfaceKey) -> bool,
        deadline: Deadline,
    ) -> Result<(), AdapterError> {
        let after = self.state.frames_sent;
        self.clear_frames(chosen);
        self.settle((after, self.state.frames_sent), deadline)
    }

    fn clear_frames(&mut self, chosen: impl Fn(&SurfaceKey) -> bool) {
        for (key, (surface, _)) in &mut self.surfaces {
            if chosen(key) {
                let frame = (surface.margin(), None);
                surface.commit_frame(&self.link, &mut self.state, frame);
            }
        }
    }

    /// Destroys the surfaces of `cursor` and their buffers.
    pub(crate) fn forget(&mut self, cursor: &str) {
        self.drop_where(|key, _| key.cursor == cursor);
        let _ = self.link.pump(&mut self.state);
    }

    fn drop_where(&mut self, chosen: impl Fn(&SurfaceKey, &CursorSurface) -> bool) {
        let keys: Vec<SurfaceKey> = self
            .surfaces
            .iter()
            .filter(|(key, (surface, _))| chosen(key, surface))
            .map(|(key, _)| key.clone())
            .collect();
        for key in keys {
            self.drop_surface(&key);
        }
    }

    fn sync_outputs(&mut self) {
        let Some(manager) = self
            .globals
            .xdg_outputs
            .clone()
            .filter(|_| self.globals.lays_out())
        else {
            return;
        };
        let advertised: Vec<Global> = self.state.exchange.all(OUTPUT).cloned().collect();
        let outputs = &mut self.state.outputs;
        let gone: Vec<TrackedOutput> = outputs
            .tracked
            .extract_if(.., |output| {
                !advertised.iter().any(|global| global.name == output.name)
            })
            .collect();
        for output in &gone {
            output.release();
            outputs.changed = true;
        }
        for global in &advertised {
            if outputs
                .tracked
                .iter()
                .any(|output| output.name == global.name)
            {
                continue;
            }
            let output: wl_output::WlOutput = self.link.bind(global, OUTPUT_VERSION);
            let xdg = manager.get_xdg_output(&output, &self.link.handle(), global.name);
            outputs.tracked.push(TrackedOutput {
                name: global.name,
                output,
                xdg,
                geometry: Default::default(),
                scale: Default::default(),
            });
            outputs.changed = true;
        }
        let closed = std::mem::take(&mut outputs.closed_layers);
        self.drop_where(|key, surface| {
            gone.iter().any(|output| key.output == Some(output.name))
                || closed.contains(&surface.layer_id())
        });
    }

    /// Destroys every surface and its buffers.
    pub(crate) fn release(&mut self) {
        for (_, (surface, _)) in std::mem::take(&mut self.surfaces) {
            surface.destroy(&mut self.state);
        }
        let _ = self.link.pump(&mut self.state);
    }

    /// How many cursor surfaces exist.
    #[cfg(test)]
    pub(crate) fn surface_count(&self) -> usize {
        self.surfaces.len()
    }

    fn drop_surface(&mut self, key: &SurfaceKey) {
        if let Some((surface, _)) = self.surfaces.remove(key) {
            surface.destroy(&mut self.state);
        }
    }

    /// Waits until the compositor has handled every request sent so far.
    pub(crate) fn roundtrip(&mut self, deadline: Deadline) -> Result<(), AdapterError> {
        self.link
            .roundtrip(&mut self.state, deadline)
            .map_err(|fault| self.error(fault))
    }

    /// Sends and dispatches what is ready without blocking, then binds the
    /// outputs that came and releases the ones that went.
    pub(crate) fn pump(&mut self) -> Result<(), AdapterError> {
        self.link
            .pump(&mut self.state)
            .map_err(|fault| self.error(fault))?;
        self.sync_outputs();
        Ok(())
    }

    /// Whether the compositor closed a cursor surface.
    pub(crate) fn is_closed(&self) -> bool {
        self.state.closed
    }

    pub(crate) fn output_count(&self) -> usize {
        output_count(&self.state)
    }

    pub(crate) fn poll_fd(&self) -> BorrowedFd<'_> {
        self.link.poll_fd()
    }

    fn error(&self, fault: WaylandFault) -> AdapterError {
        fault.into_error(&self.socket)
    }
}

#[cfg(test)]
#[path = "layer_client_tests.rs"]
mod tests;
