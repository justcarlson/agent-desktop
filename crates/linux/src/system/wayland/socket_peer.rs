//! Names the process that serves a Unix stream socket from the kernel's
//! peer credentials, before any byte is read. A credential is what the
//! kernel recorded when the listener bound, so no reply can forge it.

use std::os::unix::net::UnixStream;
use std::path::Path;

use super::socket_connect::connect;
use super::wayland_fault::WaylandFault;

/// A connected stream and the pid that serves it. A listener whose backlog
/// is full is `Silent`, because its owner cannot be read.
pub(crate) fn connect_peer(socket: &Path) -> Result<(UnixStream, u32), WaylandFault> {
    let stream = connect(socket)?;
    let pid = peer_of(&stream).ok_or(WaylandFault::Gone)?;
    Ok((stream, pid))
}

/// The pid that serves `socket`.
pub(crate) fn peer_pid(socket: &Path) -> Result<u32, WaylandFault> {
    connect_peer(socket).map(|(_, pid)| pid)
}

fn peer_of(stream: &UnixStream) -> Option<u32> {
    let credentials = rustix::net::sockopt::socket_peercred(stream).ok()?;
    let pid = rustix::process::Pid::as_raw(Some(credentials.pid));
    u32::try_from(pid).ok().filter(|pid| *pid > 0)
}

#[cfg(test)]
#[path = "socket_peer_tests.rs"]
mod tests;
