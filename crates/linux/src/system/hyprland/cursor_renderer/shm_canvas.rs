//! The pixels a cursor surface shows: one anonymous shared-memory pool that
//! holds the drawn arrow and a fully transparent frame of the same size,
//! written once with positioned writes, never mapped.

use std::fs::File;
use std::os::fd::AsFd;
use std::os::unix::fs::FileExt;

use agent_desktop_core::AdapterError;
use rustix::fs::{MemfdFlags, memfd_create};
use wayland_client::protocol::{wl_buffer, wl_shm, wl_shm_pool};

use super::arrow_bitmap::ArrowBitmap;
use super::layer_globals::LayerGlobals;
use super::layer_state::LayerState;
use wayland_client::QueueHandle;

/// The drawn and the clear buffer of one cursor.
pub(crate) struct ShmCanvas {
    pub(crate) drawn: wl_buffer::WlBuffer,
    pub(crate) clear: wl_buffer::WlBuffer,
    pub(crate) size: (i32, i32),
    pool: wl_shm_pool::WlShmPool,
    _file: File,
}

impl ShmCanvas {
    /// Writes `bitmap` and a clear frame into a new pool.
    pub(crate) fn create(
        globals: &LayerGlobals,
        bitmap: &ArrowBitmap,
        handle: &QueueHandle<LayerState>,
    ) -> Result<Self, AdapterError> {
        let frame_len = bitmap.pixels.len();
        let fail = |what: &str, detail: String| {
            AdapterError::internal(format!("Could not {what} the cursor buffer"))
                .with_platform_detail(detail)
        };
        let fd = memfd_create("agent-desktop-cursor", MemfdFlags::CLOEXEC)
            .map_err(|error| fail("allocate", error.to_string()))?;
        let file = File::from(fd);
        let total = frame_len * 2;
        file.set_len(total as u64)
            .and_then(|()| file.write_all_at(&bitmap.pixels, 0))
            .map_err(|error| fail("write", error.to_string()))?;
        let geometry = |value: u32| {
            i32::try_from(value).map_err(|_| fail("size", format!("{value} does not fit")))
        };
        let width = geometry(bitmap.width)?;
        let height = geometry(bitmap.height)?;
        let stride = geometry(bitmap.stride())?;
        let pool_len = i32::try_from(total).map_err(|_| fail("size", total.to_string()))?;
        let offset = i32::try_from(frame_len).map_err(|_| fail("size", frame_len.to_string()))?;
        let pool = globals.shm.create_pool(file.as_fd(), pool_len, handle, ());
        let format = wl_shm::Format::Argb8888;
        let drawn = pool.create_buffer(0, width, height, stride, format, handle, ());
        let clear = pool.create_buffer(offset, width, height, stride, format, handle, ());
        Ok(Self {
            drawn,
            clear,
            size: (width, height),
            pool,
            _file: file,
        })
    }

    /// Destroys both buffers and the pool.
    pub(crate) fn destroy(&self) {
        self.drawn.destroy();
        self.clear.destroy();
        self.pool.destroy();
    }
}
