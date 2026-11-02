//! What the renderer watches of its compositor: the instance's runtime
//! directory, whose loss means the compositor is gone, and its event socket.
//! Hyprland 0.56.2 clears runtime layer rules when it reloads its config, and
//! mapped cursor surfaces then animate again; a `no_anim` rule added after
//! the reload reaches them without a remap (probe row L43-7). Hyprland
//! reloads when its config file changes and Omarchy reloads on a theme
//! switch, so on every `configreloaded` event the renderer asks for the rule
//! again before it draws the next frame. The event socket is read only when
//! the bound compositor process serves it; without it the renderer keeps the
//! rule it asked for at start.

use std::io::{ErrorKind, Read};
use std::os::fd::{AsFd, BorrowedFd};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use super::layer_animation::suppress_layer_animation;
use super::renderer_target::RendererTarget;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::wayland::socket_peer::connect_peer;

const EVENT_SOCKET: &str = ".socket2.sock";
const RELOADED: &[u8] = b"configreloaded>>";
const LINE_CAP: usize = 4096;

/// The renderer's view of its compositor instance.
pub(crate) struct CompositorWatch {
    dir: PathBuf,
    binding: HyprlandBinding,
    events: Option<UnixStream>,
    partial: Vec<u8>,
}

impl CompositorWatch {
    /// Listens for config reloads, then asks the compositor not to animate
    /// cursor surfaces, so a reload between the two is not missed.
    pub(crate) fn open(target: &RendererTarget) -> Self {
        let binding = target.binding.clone();
        let events = connect_peer(&binding.ipc_socket.with_file_name(EVENT_SOCKET))
            .ok()
            .filter(|(_, pid)| *pid == binding.pid)
            .and_then(|(stream, _)| stream.set_nonblocking(true).ok().map(|()| stream));
        suppress_layer_animation(&binding);
        Self {
            dir: target.runtime_dir.join("hypr").join(&binding.signature),
            binding,
            events,
            partial: Vec::new(),
        }
    }

    /// Whether the compositor instance's runtime directory is still there.
    pub(crate) fn is_present(&self) -> bool {
        self.dir.is_dir()
    }

    /// The event socket, while it is open.
    pub(crate) fn poll_fd(&self) -> Option<BorrowedFd<'_>> {
        self.events.as_ref().map(AsFd::as_fd)
    }

    /// Reads every pending event and asks again for the `no_anim` rule if
    /// the compositor reloaded its config since the last read.
    pub(crate) fn reapply_after_reload(&mut self) {
        if self.read_pending() {
            suppress_layer_animation(&self.binding);
        }
    }

    fn read_pending(&mut self) -> bool {
        let Some(events) = self.events.as_mut() else {
            return false;
        };
        let mut reloaded = false;
        let mut chunk = [0_u8; 4096];
        loop {
            match events.read(&mut chunk) {
                Ok(0) => {
                    self.events = None;
                    return reloaded;
                }
                Ok(read) => reloaded |= reload_in(&mut self.partial, &chunk[..read]),
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) if error.kind() == ErrorKind::WouldBlock => return reloaded,
                Err(_) => {
                    self.events = None;
                    return reloaded;
                }
            }
        }
    }
}

/// Appends `chunk` to the unfinished line in `partial` and returns whether
/// any line it completes is a `configreloaded` event. A line longer than
/// `LINE_CAP` is no event the renderer reads, so its bytes are dropped and a
/// NUL stands in for them until its newline comes.
pub(crate) fn reload_in(partial: &mut Vec<u8>, chunk: &[u8]) -> bool {
    partial.extend_from_slice(chunk);
    let complete = partial.iter().rposition(|byte| *byte == b'\n');
    let reloaded = complete.is_some_and(|end| {
        partial[..end]
            .split(|byte| *byte == b'\n')
            .any(|line| line.starts_with(RELOADED))
    });
    if let Some(end) = complete {
        partial.drain(..=end);
    }
    if partial.len() > LINE_CAP {
        partial.clear();
        partial.push(0);
    }
    reloaded
}

#[cfg(test)]
#[path = "compositor_watch_tests.rs"]
mod tests;
