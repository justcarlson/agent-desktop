//! The pixels a cursor surface shows: one anonymous shared-memory pool that
//! holds a fully transparent frame and the arrow drawn at every whole
//! physical offset a placement can ask for, written once with positioned
//! writes, never mapped.

use std::fs::File;
use std::os::fd::AsFd;
use std::os::unix::fs::FileExt;

use agent_desktop_core::AdapterError;
use rustix::fs::{MemfdFlags, memfd_create};
use wayland_client::QueueHandle;
use wayland_client::protocol::{wl_buffer, wl_shm, wl_shm_pool};

use super::arrow_bitmap::ArrowBitmap;
use super::layer_globals::LayerGlobals;
use super::layer_state::LayerState;
use super::placement::{Placement, reach};

/// The clear buffer and the drawn buffers of one cursor surface.
pub(crate) struct ShmCanvas {
    pub(crate) clear: wl_buffer::WlBuffer,
    drawn: Vec<wl_buffer::WlBuffer>,
    pub(crate) size: (i32, i32),
    reach: i32,
    pool: wl_shm_pool::WlShmPool,
    _file: File,
}

fn fail(what: &str, detail: String) -> AdapterError {
    AdapterError::internal(format!("Could not {what} the cursor buffer"))
        .with_platform_detail(detail)
}

fn fits(value: usize) -> Result<i32, AdapterError> {
    i32::try_from(value).map_err(|_| fail("size", format!("{value} does not fit")))
}

impl ShmCanvas {
    /// Writes a clear frame and `bitmap` at each offset into a new pool of
    /// `placement`'s buffer size.
    pub(crate) fn create(
        globals: &LayerGlobals,
        bitmap: &ArrowBitmap,
        placement: &Placement,
        handle: &QueueHandle<LayerState>,
    ) -> Result<Self, AdapterError> {
        let (width, height) = placement.buffer;
        let size = (width.max(1), height.max(1));
        let reach = reach(placement.scale);
        let frame_len = size.0 as usize * size.1 as usize * 4;
        let variants = ((reach + 1) * (reach + 1)) as usize;
        let total = frame_len * (variants + 1);
        let fd = memfd_create("agent-desktop-cursor", MemfdFlags::CLOEXEC)
            .map_err(|error| fail("allocate", error.to_string()))?;
        let file = File::from(fd);
        file.set_len(total as u64)
            .map_err(|error| fail("write", error.to_string()))?;
        for index in 0..variants {
            let offset = (index as i32 % (reach + 1), index as i32 / (reach + 1));
            let frame = compose(bitmap, size, offset);
            file.write_all_at(&frame, (frame_len * (index + 1)) as u64)
                .map_err(|error| fail("write", error.to_string()))?;
        }
        let pool = globals
            .shm
            .create_pool(file.as_fd(), fits(total)?, handle, ());
        let format = wl_shm::Format::Argb8888;
        let stride = size.0 * 4;
        let buffer = |at: usize| -> Result<wl_buffer::WlBuffer, AdapterError> {
            let start = fits(at)?;
            Ok(pool.create_buffer(start, size.0, size.1, stride, format, handle, ()))
        };
        let clear = buffer(0)?;
        let drawn = (0..variants)
            .map(|index| buffer(frame_len * (index + 1)))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            clear,
            drawn,
            size,
            reach,
            pool,
            _file: file,
        })
    }

    /// The buffer that shows the arrow at `offset`, or the clear one.
    pub(crate) fn frame(&self, offset: Option<(i32, i32)>) -> &wl_buffer::WlBuffer {
        let Some((x, y)) = offset else {
            return &self.clear;
        };
        let side = self.reach + 1;
        let index = y.clamp(0, self.reach) * side + x.clamp(0, self.reach);
        self.drawn.get(index as usize).unwrap_or(&self.clear)
    }

    /// Destroys every buffer and the pool.
    pub(crate) fn destroy(&self) {
        self.clear.destroy();
        self.drawn.iter().for_each(wl_buffer::WlBuffer::destroy);
        self.pool.destroy();
    }
}

fn compose(bitmap: &ArrowBitmap, size: (i32, i32), offset: (i32, i32)) -> Vec<u8> {
    let (width, height) = (size.0 as usize, size.1 as usize);
    let (left, top) = (offset.0.max(0) as usize, offset.1.max(0) as usize);
    let mut frame = vec![0_u8; width * height * 4];
    let source_stride = bitmap.stride() as usize;
    let columns = (bitmap.width as usize).min(width.saturating_sub(left));
    for row in 0..(bitmap.height as usize).min(height.saturating_sub(top)) {
        let from = row * source_stride;
        let to = ((row + top) * width + left) * 4;
        frame[to..to + columns * 4].copy_from_slice(&bitmap.pixels[from..from + columns * 4]);
    }
    frame
}

#[cfg(test)]
#[path = "shm_canvas_tests.rs"]
mod tests;
