//! One request on a Hyprland IPC socket (`.socket.sock`): write the command,
//! then read the reply until the compositor closes the connection. Every
//! write and read waits for at most the time left on the deadline.

use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use agent_desktop_core::Deadline;

pub(crate) use super::ipc_fault::IpcFault;
pub(crate) use super::ipc_reply::IpcReply;
use crate::system::wayland::peer_fault::PeerFault;
use crate::system::wayland::socket_peer::connect;

/// The largest reply read. A monitor or client list of a real session is a
/// few kilobytes.
pub(crate) const REPLY_CAP: usize = 1024 * 1024;

const CHUNK: usize = 16 * 1024;

/// Sends `command` (for example `j/monitors`) and returns the whole reply.
pub(crate) fn request(
    socket: &Path,
    command: &str,
    deadline: Deadline,
) -> Result<IpcReply, IpcFault> {
    let (mut stream, pid) = connect(socket).map_err(|fault| match fault {
        PeerFault::Gone => IpcFault::Gone,
        PeerFault::Busy => IpcFault::Silent,
    })?;
    limit(&stream, deadline)?;
    stream
        .write_all(command.as_bytes())
        .map_err(|error| io_fault(error.kind()))?;
    let mut body = Vec::new();
    let mut chunk = vec![0_u8; CHUNK];
    loop {
        limit(&stream, deadline)?;
        match stream.read(&mut chunk) {
            Ok(0) if body.is_empty() => return Err(IpcFault::Gone),
            Ok(0) => return Ok(IpcReply { pid, body }),
            Ok(read) => {
                if body.len() + read > REPLY_CAP {
                    return Err(IpcFault::Oversized);
                }
                body.extend_from_slice(&chunk[..read]);
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) => return Err(io_fault(error.kind())),
        }
    }
}

fn limit(stream: &UnixStream, deadline: Deadline) -> Result<(), IpcFault> {
    let remaining = deadline.remaining();
    if remaining < Duration::from_millis(1) {
        return Err(IpcFault::Silent);
    }
    stream
        .set_read_timeout(Some(remaining))
        .and_then(|()| stream.set_write_timeout(Some(remaining)))
        .map_err(|_| IpcFault::Gone)
}

fn io_fault(kind: ErrorKind) -> IpcFault {
    match kind {
        ErrorKind::WouldBlock | ErrorKind::TimedOut => IpcFault::Silent,
        _ => IpcFault::Gone,
    }
}

#[cfg(test)]
#[path = "ipc_tests.rs"]
mod tests;
