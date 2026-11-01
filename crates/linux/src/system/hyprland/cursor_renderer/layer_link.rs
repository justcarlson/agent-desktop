//! The renderer's one long-lived compositor connection. Unlike a command's
//! link, every wait takes its own deadline, and an idle loop can poll the
//! connection's descriptor beside the control socket and pump what arrived.

use std::os::fd::{AsFd, BorrowedFd};
use std::os::unix::net::UnixStream;

use agent_desktop_core::Deadline;
use rustix::event::{PollFd, PollFlags, Timespec};
use wayland_client::backend::WaylandError;
use wayland_client::protocol::wl_registry;
use wayland_client::{Connection, EventQueue, Proxy, QueueHandle};

use super::layer_state::LayerState;
use crate::system::wayland::dispatch::dispatch_until;
use crate::system::wayland::exchange::HoldsExchange;
use crate::system::wayland::global::Global;
use crate::system::wayland::wayland_fault::WaylandFault;

/// An open connection, its queue and registry, and the round trips so far.
pub(crate) struct LayerLink {
    connection: Connection,
    queue: EventQueue<LayerState>,
    registry: wl_registry::WlRegistry,
    rounds: u32,
}

impl LayerLink {
    /// Reads the registry over `stream` into `state`.
    pub(crate) fn over(
        stream: UnixStream,
        state: &mut LayerState,
        deadline: Deadline,
    ) -> Result<Self, WaylandFault> {
        let connection = Connection::from_socket(stream).map_err(|_| WaylandFault::Gone)?;
        let queue = connection.new_event_queue::<LayerState>();
        let registry = connection.display().get_registry(&queue.handle(), ());
        let mut link = Self {
            connection,
            queue,
            registry,
            rounds: 0,
        };
        link.roundtrip(state, deadline)?;
        Ok(link)
    }

    /// Waits until the compositor has answered every request sent so far.
    pub(crate) fn roundtrip(
        &mut self,
        state: &mut LayerState,
        deadline: Deadline,
    ) -> Result<(), WaylandFault> {
        self.rounds += 1;
        let round = self.rounds;
        let _callback = self.connection.display().sync(&self.queue.handle(), round);
        self.wait(
            state,
            |state| state.exchange().has_finished(round),
            deadline,
        )
    }

    /// Dispatches events until `done` holds or `deadline` passes.
    pub(crate) fn wait(
        &mut self,
        state: &mut LayerState,
        done: impl Fn(&LayerState) -> bool,
        deadline: Deadline,
    ) -> Result<(), WaylandFault> {
        dispatch_until(&mut self.queue, state, done, deadline)
    }

    /// Sends what is queued, reads what has arrived and dispatches it,
    /// without blocking.
    pub(crate) fn pump(&mut self, state: &mut LayerState) -> Result<(), WaylandFault> {
        self.queue
            .dispatch_pending(state)
            .map_err(|_| WaylandFault::Gone)?;
        match self.connection.flush() {
            Ok(()) => {}
            Err(WaylandError::Io(io)) if io.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(WaylandError::Protocol(protocol)) => {
                return Err(WaylandFault::Protocol(protocol.to_string()));
            }
            Err(WaylandError::Io(_)) => return Err(WaylandFault::Gone),
        }
        let Some(guard) = self.queue.prepare_read() else {
            return self.dispatch(state);
        };
        let ready = {
            let fd = guard.connection_fd();
            let mut fds = [PollFd::new(&fd, PollFlags::IN)];
            rustix::event::poll(&mut fds, Some(&Timespec::default())).unwrap_or(0) > 0
        };
        if ready {
            match guard.read() {
                Ok(_) => {}
                Err(WaylandError::Io(io)) if io.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(WaylandError::Protocol(protocol)) => {
                    return Err(WaylandFault::Protocol(protocol.to_string()));
                }
                Err(WaylandError::Io(_)) => return Err(WaylandFault::Gone),
            }
        }
        self.dispatch(state)
    }

    fn dispatch(&mut self, state: &mut LayerState) -> Result<(), WaylandFault> {
        self.queue
            .dispatch_pending(state)
            .map(|_| ())
            .map_err(|_| WaylandFault::Gone)
    }

    /// Binds `global` at the lower of its advertised version and `version`.
    pub(crate) fn bind<I>(&self, global: &Global, version: u32) -> I
    where
        I: Proxy + 'static,
        LayerState: wayland_client::Dispatch<I, ()>,
    {
        self.registry.bind::<I, (), LayerState>(
            global.name,
            global.version.min(version),
            &self.queue.handle(),
            (),
        )
    }

    pub(crate) fn handle(&self) -> QueueHandle<LayerState> {
        self.queue.handle()
    }

    /// The connection's descriptor, for an idle loop's `poll`.
    pub(crate) fn poll_fd(&self) -> BorrowedFd<'_> {
        self.connection.as_fd()
    }
}
