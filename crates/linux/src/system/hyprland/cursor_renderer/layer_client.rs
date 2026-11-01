//! The renderer's drawing side: one connection to the bound compositor, the
//! globals it binds, and one surface per cursor key, each created on its
//! first draw. Every agent cursor draws on its own surfaces, so one agent's
//! style or position never rebuilds or moves another's.

use std::collections::BTreeMap;
use std::os::fd::BorrowedFd;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use agent_desktop_core::{AdapterError, Deadline};

use super::arrow_bitmap::ArrowBitmap;
use super::cursor_surface::{CursorSurface, settle};
use super::layer_globals::{LayerGlobals, output_count};
use super::layer_link::LayerLink;
use super::layer_state::LayerState;
use super::shm_canvas::ShmCanvas;
use super::surface_key::SurfaceKey;
use crate::system::wayland::wayland_fault::WaylandFault;

/// A connection that draws the session's cursors.
pub(crate) struct LayerClient {
    link: LayerLink,
    state: LayerState,
    globals: LayerGlobals,
    surfaces: BTreeMap<SurfaceKey, (CursorSurface, ArrowBitmap)>,
    socket: PathBuf,
}

impl LayerClient {
    /// Reads the registry over `stream`, a connection to the compositor at
    /// `socket`, and binds the drawing globals. No surface exists yet.
    pub(crate) fn over(
        stream: UnixStream,
        socket: &Path,
        deadline: Deadline,
    ) -> Result<Self, AdapterError> {
        let mut state = LayerState::default();
        let link = LayerLink::over(stream, &mut state, deadline)
            .map_err(|fault| fault.into_error(socket))?;
        let globals = LayerGlobals::bind(&link, &state)?;
        Ok(Self {
            link,
            state,
            globals,
            surfaces: BTreeMap::new(),
            socket: socket.to_path_buf(),
        })
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
        let mark = self.stage(key, bitmap, at, deadline)?;
        self.settle(mark, deadline)
    }

    /// Commits `bitmap` on the surface `key` at `at` without waiting for
    /// the frame, creating or rebuilding that surface when its bitmap
    /// changed, and returns the frame mark to settle on.
    pub(crate) fn stage(
        &mut self,
        key: &SurfaceKey,
        bitmap: &ArrowBitmap,
        at: (i32, i32),
        deadline: Deadline,
    ) -> Result<u32, AdapterError> {
        if self.surfaces.get(key).map(|(_, drawn)| drawn) != Some(bitmap) {
            self.drop_surface(key);
            let canvas = ShmCanvas::create(&self.globals, bitmap, &self.link.handle())?;
            let surface = CursorSurface::create(
                &mut self.link,
                &mut self.state,
                &self.globals,
                canvas,
                deadline,
            )
            .map_err(|fault| self.error(fault))?;
            self.surfaces.insert(key.clone(), (surface, bitmap.clone()));
        }
        let Some((surface, _)) = self.surfaces.get_mut(key) else {
            return Ok(self.state.frames_sent);
        };
        Ok(surface.commit_frame(&self.link, &mut self.state, (at.0, at.1, true)))
    }

    /// Waits until the frame `mark` names has shown, or the compositor
    /// shows nothing until `deadline`.
    pub(crate) fn settle(&mut self, mark: u32, deadline: Deadline) -> Result<(), AdapterError> {
        settle(&mut self.link, &mut self.state, mark, deadline).map_err(|fault| self.error(fault))
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
        let mut last = None;
        for (key, (surface, _)) in &mut self.surfaces {
            if chosen(key) {
                let (left, top) = surface.margin();
                last = Some(surface.commit_frame(&self.link, &mut self.state, (left, top, false)));
            }
        }
        match last {
            Some(mark) => self.settle(mark, deadline),
            None => Ok(()),
        }
    }

    /// Destroys the surfaces of `cursor` and their buffers.
    pub(crate) fn forget(&mut self, cursor: &str) {
        let keys: Vec<SurfaceKey> = self
            .surfaces
            .keys()
            .filter(|key| key.cursor == cursor)
            .cloned()
            .collect();
        for key in keys {
            self.drop_surface(&key);
        }
        let _ = self.link.pump(&mut self.state);
    }

    /// Destroys every surface and its buffers.
    pub(crate) fn release(&mut self) {
        for (_, (surface, _)) in std::mem::take(&mut self.surfaces) {
            surface.destroy();
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
            surface.destroy();
        }
    }

    /// Waits until the compositor has handled every request sent so far.
    pub(crate) fn roundtrip(&mut self, deadline: Deadline) -> Result<(), AdapterError> {
        self.link
            .roundtrip(&mut self.state, deadline)
            .map_err(|fault| self.error(fault))
    }

    /// Sends and dispatches what is ready without blocking.
    pub(crate) fn pump(&mut self) -> Result<(), AdapterError> {
        self.link
            .pump(&mut self.state)
            .map_err(|fault| self.error(fault))
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
