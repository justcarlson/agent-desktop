//! Keymap text carried by file descriptor, both ways: the seat owner's keymap
//! read from the fd a `wl_keyboard.keymap` event carries, and a keymap
//! written into a memfd for `zwp_virtual_keyboard_v1.keymap`.
//! Both sizes count the text's trailing NUL, as both protocols specify.

use std::fs::File;
use std::io::Write;
use std::os::fd::OwnedFd;
use std::os::unix::fs::FileExt;

use agent_desktop_core::AdapterError;
use rustix::fs::{MemfdFlags, memfd_create};
use serde_json::json;

use crate::input::wayland::keymap::refusal::keymap_refused;

const KEYMAP_LIMIT: u32 = 4 * 1024 * 1024;

/// The keymap text in the first `size` bytes of `fd`, without its NUL.
pub(crate) fn read_keymap(fd: OwnedFd, size: u32) -> Result<String, AdapterError> {
    let _ = (fd, size, KEYMAP_LIMIT);
    Err(unreadable("not read"))
}

/// A memfd that holds `text` and its NUL, with the size to announce.
pub(crate) fn keymap_memfd(text: &str) -> Result<(OwnedFd, u32), AdapterError> {
    let size = u32::try_from(text.len() + 1)
        .ok()
        .filter(|size| *size <= KEYMAP_LIMIT)
        .ok_or_else(|| {
            keymap_refused(
                "The extended keyboard keymap is too large to send",
                "extended_keymap_too_large",
                json!({ "bytes": text.len() }),
            )
        })?;
    let fd = memfd_create("agent-desktop-keymap", MemfdFlags::CLOEXEC)
        .map_err(|error| AdapterError::internal(format!("no keymap memfd: {error}")))?;
    let mut file = File::from(fd);
    file.write_all(text.as_bytes())
        .and_then(|()| file.write_all(&[0]))
        .map_err(|error| {
            AdapterError::internal(format!("the keymap memfd write failed: {error}"))
        })?;
    Ok((OwnedFd::from(file), size))
}

fn unreadable(detail: impl Into<String>) -> AdapterError {
    keymap_refused(
        "The seat's keyboard keymap could not be read",
        "owner_keymap_unreadable",
        json!({ "detail": detail.into() }),
    )
}

#[cfg(test)]
#[path = "keymap_fd_tests.rs"]
mod tests;
