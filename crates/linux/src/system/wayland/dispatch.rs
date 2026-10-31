//! Event dispatch with a deadline. Every read waits in `poll` for at most the
//! time left, so a compositor that accepts the connection and never answers
//! ends the exchange as `Silent` instead of blocking the command.

use std::io::ErrorKind;

use agent_desktop_core::Deadline;
use rustix::event::{PollFd, PollFlags, Timespec};
use wayland_client::backend::WaylandError;
use wayland_client::{DispatchError, EventQueue};

use super::wayland_fault::WaylandFault;

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
            Err(WaylandError::Io(error)) if error.kind() == ErrorKind::WouldBlock => {}
            Err(_) => return Err(WaylandFault::Gone),
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
        let polled = rustix::event::poll(&mut fds, Some(&timeout));
        match polled {
            Ok(0) => return Err(WaylandFault::Silent),
            Ok(_) => {}
            Err(rustix::io::Errno::INTR) => continue,
            Err(_) => return Err(WaylandFault::Gone),
        }
        match guard.read() {
            Ok(_) => {}
            Err(WaylandError::Io(error)) if error.kind() == ErrorKind::WouldBlock => {}
            Err(_) => return Err(WaylandFault::Gone),
        }
    }
}

fn dispatch_fault(_error: DispatchError) -> WaylandFault {
    WaylandFault::Gone
}
