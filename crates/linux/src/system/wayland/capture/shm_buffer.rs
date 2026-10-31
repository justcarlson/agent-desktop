//! One frame buffer in anonymous shared memory that only this process and
//! the compositor hold. The `wl_buffer` is destroyed when the buffer drops,
//! on every path, so a failed or abandoned capture leaves nothing behind.

use std::fs::File;
use std::os::fd::AsFd;
use std::os::unix::fs::FileExt;

use rustix::fs::{MemfdFlags, memfd_create};
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_shm::{self, WlShm};
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::{Dispatch, QueueHandle};

use super::buffer_layout::BufferLayout;
use crate::system::capture::pixels::MAX_PIXELS;

/// A shared-memory frame buffer and the `wl_buffer` that names it.
pub(crate) struct ShmBuffer {
    file: File,
    buffer: WlBuffer,
    layout: BufferLayout,
}

impl ShmBuffer {
    /// Allocates a buffer for `layout`, or `None` when the layout is empty,
    /// too large, has a stride shorter than a row, or names a format the
    /// protocol does not know.
    pub(crate) fn create<D>(
        shm: &WlShm,
        layout: BufferLayout,
        handle: &QueueHandle<D>,
    ) -> Option<Self>
    where
        D: Dispatch<WlShmPool, ()> + Dispatch<WlBuffer, ()> + 'static,
    {
        let len = byte_len(layout)?;
        let format = wl_shm::Format::try_from(layout.format).ok()?;
        let fd = memfd_create("agent-desktop-capture", MemfdFlags::CLOEXEC).ok()?;
        let file = File::from(fd);
        file.set_len(u64::try_from(len).ok()?).ok()?;
        let pool = shm.create_pool(file.as_fd(), len, handle, ());
        let buffer = pool.create_buffer(
            0,
            i32::try_from(layout.width).ok()?,
            i32::try_from(layout.height).ok()?,
            i32::try_from(layout.stride).ok()?,
            format,
            handle,
            (),
        );
        pool.destroy();
        Some(Self {
            file,
            buffer,
            layout,
        })
    }

    pub(crate) fn buffer(&self) -> &WlBuffer {
        &self.buffer
    }

    /// The bytes the compositor wrote.
    pub(crate) fn read(&self) -> Option<Vec<u8>> {
        let len = usize::try_from(byte_len(self.layout)?).ok()?;
        let mut data = vec![0; len];
        self.file.read_exact_at(&mut data, 0).ok()?;
        Some(data)
    }
}

impl Drop for ShmBuffer {
    fn drop(&mut self) {
        self.buffer.destroy();
    }
}

fn byte_len(layout: BufferLayout) -> Option<i32> {
    let row = layout.width.checked_mul(4)?;
    if layout.width == 0 || layout.height == 0 || layout.stride < row {
        return None;
    }
    if u64::from(layout.width) * u64::from(layout.height) > MAX_PIXELS {
        return None;
    }
    i32::try_from(layout.stride.checked_mul(layout.height)?).ok()
}

#[cfg(test)]
#[path = "shm_buffer_tests.rs"]
mod tests;
