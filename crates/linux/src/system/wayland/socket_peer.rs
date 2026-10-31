//! Connects to a Unix stream socket without blocking and names the process
//! that serves it from the kernel's peer credentials, before any byte is
//! read. A credential is what the kernel recorded when the listener bound,
//! so a reply cannot forge it.

use std::os::unix::net::UnixStream;
use std::path::Path;

use rustix::io::Errno;
use rustix::net::{AddressFamily, SocketAddrUnix, SocketFlags, SocketType};

use super::peer_fault::PeerFault;

/// A connected stream in blocking mode and the pid that serves it.
pub(crate) fn connect(socket: &Path) -> Result<(UnixStream, u32), PeerFault> {
    let address = SocketAddrUnix::new(socket).map_err(|_| PeerFault::Gone)?;
    let fd = rustix::net::socket_with(
        AddressFamily::UNIX,
        SocketType::STREAM,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )
    .map_err(|_| PeerFault::Gone)?;
    match rustix::net::connect(&fd, &address) {
        Ok(()) => {}
        Err(Errno::AGAIN) => return Err(PeerFault::Busy),
        Err(_) => return Err(PeerFault::Gone),
    }
    let credentials = rustix::net::sockopt::socket_peercred(&fd).map_err(|_| PeerFault::Gone)?;
    let pid = rustix::process::Pid::as_raw(Some(credentials.pid));
    let pid = u32::try_from(pid)
        .ok()
        .filter(|pid| *pid > 0)
        .ok_or(PeerFault::Gone)?;
    let stream = UnixStream::from(fd);
    stream.set_nonblocking(false).map_err(|_| PeerFault::Gone)?;
    Ok((stream, pid))
}

/// The pid that serves a socket, or `None` when nothing serves it.
pub(crate) fn peer_pid(socket: &Path) -> Option<u32> {
    connect(socket).ok().map(|(_, pid)| pid)
}

#[cfg(test)]
#[path = "socket_peer_tests.rs"]
mod tests;
