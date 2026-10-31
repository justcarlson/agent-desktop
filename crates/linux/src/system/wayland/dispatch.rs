//! Event dispatch with a deadline. Every read waits in `poll` for at most the
//! time left, so a compositor that accepts the connection and never answers
//! ends the exchange as `Silent` instead of blocking the command.

use std::io::ErrorKind;

use agent_desktop_core::Deadline;
use rustix::event::{PollFd, PollFlags, Timespec};
use wayland_client::backend::WaylandError;
use wayland_client::{DispatchError, EventQueue};

use super::wayland_fault::WaylandFault;

fn backend_fault(error: WaylandError) -> WaylandFault {
    match error {
        WaylandError::Io(_) => WaylandFault::Gone,
        WaylandError::Protocol(protocol) => WaylandFault::Protocol(protocol.to_string()),
    }
}

fn dispatch_fault(error: DispatchError) -> WaylandFault {
    match error {
        DispatchError::Backend(backend) => backend_fault(backend),
        DispatchError::BadMessage { .. } => WaylandFault::Protocol(error.to_string()),
    }
}

fn would_block(error: &WaylandError) -> bool {
    matches!(error, WaylandError::Io(io) if io.kind() == ErrorKind::WouldBlock)
}

/// Dispatches events until `done` holds or the deadline passes.
pub(crate) fn dispatch_until<S>(
    queue: &mut EventQueue<S>,
    state: &mut S,
    done: impl Fn(&S) -> bool,
    deadline: Deadline,
) -> Result<(), WaylandFault> {
    loop {
        queue.dispatch_pending(state).map_err(dispatch_fault)?;
        if done(state) {
            return Ok(());
        }
        match queue.flush() {
            Ok(()) => {}
            Err(error) if would_block(&error) => {}
            Err(error) => return Err(backend_fault(error)),
        }
        let Some(guard) = queue.prepare_read() else {
            continue;
        };
        let remaining = deadline.remaining();
        if remaining.is_zero() {
            return Err(WaylandFault::Silent);
        }
        let timeout = Timespec::try_from(remaining).map_err(|_| WaylandFault::Silent)?;
        let connection_fd = guard.connection_fd();
        let mut fds = [PollFd::new(&connection_fd, PollFlags::IN)];
        match rustix::event::poll(&mut fds, Some(&timeout)) {
            Ok(0) => return Err(WaylandFault::Silent),
            Ok(_) => {}
            Err(rustix::io::Errno::INTR) => continue,
            Err(_) => return Err(WaylandFault::Gone),
        }
        match guard.read() {
            Ok(_) => {}
            Err(error) if would_block(&error) => {}
            Err(error) => return Err(backend_fault(error)),
        }
    }
}
