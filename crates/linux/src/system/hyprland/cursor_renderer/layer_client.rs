//! The renderer's drawing side: one connection to the bound compositor, the
//! globals it binds, and the cursor surface, created on the first draw.

use std::os::fd::BorrowedFd;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use agent_desktop_core::{AdapterError, Deadline};

use super::arrow_bitmap::ArrowBitmap;
use super::cursor_surface::CursorSurface;
use super::layer_globals::{LayerGlobals, output_count};
use super::layer_link::LayerLink;
use super::layer_state::LayerState;
use super::shm_canvas::ShmCanvas;
use crate::system::wayland::wayland_fault::WaylandFault;

/// A connection that draws one cursor.
pub(crate) struct LayerClient {
    link: LayerLink,
    state: LayerState,
    globals: LayerGlobals,
    surface: Option<CursorSurface>,
    bitmap: Option<ArrowBitmap>,
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
        let _ = (stream, socket, deadline);
        return Err(AdapterError::internal("not implemented"));
        #[allow(unreachable_code)]
        let mut state = LayerState::default();
        let link = LayerLink::over(stream, &mut state, deadline)
            .map_err(|fault| fault.into_error(socket))?;
        let globals = LayerGlobals::bind(&link, &state)?;
        Ok(Self {
            link,
            state,
            globals,
            surface: None,
            bitmap: None,
            socket: socket.to_path_buf(),
        })
    }

    /// Shows `bitmap` with its top-left corner at `at`, creating or
    /// rebuilding the surface when the bitmap changed.
    pub(crate) fn draw(
        &mut self,
        bitmap: &ArrowBitmap,
        at: (i32, i32),
        deadline: Deadline,
    ) -> Result<(), AdapterError> {
        if self.bitmap.as_ref() != Some(bitmap) {
            self.release();
            let canvas = ShmCanvas::create(&self.globals, bitmap, &self.link.handle())?;
            let surface = CursorSurface::create(
                &mut self.link,
                &mut self.state,
                &self.globals,
                canvas,
                deadline,
            )
            .map_err(|fault| self.error(fault))?;
            self.surface = Some(surface);
            self.bitmap = Some(bitmap.clone());
        }
        self.present((at.0, at.1, true), deadline)
    }

    /// Commits the clear frame where the cursor is, so nothing shows.
    pub(crate) fn clear(&mut self, deadline: Deadline) -> Result<(), AdapterError> {
        let Some(at) = self.surface.as_ref().map(CursorSurface::margin) else {
            return Ok(());
        };
        self.present((at.0, at.1, false), deadline)
    }

    fn present(&mut self, frame: (i32, i32, bool), deadline: Deadline) -> Result<(), AdapterError> {
        let Some(surface) = self.surface.as_mut() else {
            return Ok(());
        };
        let shown = surface.show(&mut self.link, &mut self.state, frame, deadline);
        shown.map_err(|fault| self.error(fault))
    }

    /// Destroys the surface and its buffers.
    pub(crate) fn release(&mut self) {
        if let Some(surface) = self.surface.take() {
            surface.destroy();
        }
        self.bitmap = None;
        let _ = self.link.pump(&mut self.state);
    }

    /// Sends and dispatches what is ready without blocking.
    /// Waits until the compositor has handled every request sent so far.
    #[cfg(test)]
    pub(crate) fn roundtrip(&mut self, deadline: Deadline) -> Result<(), AdapterError> {
        self.link
            .roundtrip(&mut self.state, deadline)
            .map_err(|fault| self.error(fault))
    }

    pub(crate) fn pump(&mut self) -> Result<(), AdapterError> {
        self.link
            .pump(&mut self.state)
            .map_err(|fault| self.error(fault))
    }

    /// Whether the compositor closed the cursor surface.
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
