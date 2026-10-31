//! Connects to a compositor's Unix stream socket without blocking, so a
//! listener whose backlog is full ends the attempt instead of the command.

use std::os::unix::net::UnixStream;
use std::path::Path;

use rustix::io::Errno;
use rustix::net::{AddressFamily, SocketAddrUnix, SocketFlags, SocketType};

use super::wayland_fault::WaylandFault;

/// A connected stream in blocking mode. Nothing listening is `Gone`; a
/// listener that cannot take the connection now is `Silent`.
pub(crate) fn connect(socket: &Path) -> Result<UnixStream, WaylandFault> {
    let address = SocketAddrUnix::new(socket).map_err(|_| WaylandFault::Gone)?;
    let fd = rustix::net::socket_with(
        AddressFamily::UNIX,
        SocketType::STREAM,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )
    .map_err(|_| WaylandFault::Gone)?;
    match rustix::net::connect(&fd, &address) {
        Ok(()) => {}
        Err(Errno::AGAIN) => return Err(WaylandFault::Silent),
        Err(_) => return Err(WaylandFault::Gone),
    }
    let stream = UnixStream::from(fd);
    stream
        .set_nonblocking(false)
        .map_err(|_| WaylandFault::Gone)?;
    Ok(stream)
}

#[cfg(test)]
#[path = "socket_connect_tests.rs"]
mod tests;
