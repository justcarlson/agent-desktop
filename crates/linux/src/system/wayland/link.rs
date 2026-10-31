//! One connection to a compositor: opened on an explicit socket, its
//! registry read in a first round trip, and every later round trip bounded by
//! the same deadline.

use std::os::unix::net::UnixStream;
use std::path::Path;

use agent_desktop_core::Deadline;
use wayland_client::protocol::{wl_callback, wl_registry};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};

use super::dispatch::dispatch_until;
use super::exchange::HoldsExchange;
use super::global::Global;
use super::socket_connect::connect;
use super::wayland_fault::WaylandFault;

/// An open connection with its event queue and registry.
pub(crate) struct Link<S> {
    connection: Connection,
    queue: EventQueue<S>,
    registry: wl_registry::WlRegistry,
    rounds: u32,
    deadline: Deadline,
}

impl<S> Link<S>
where
    S: Dispatch<wl_registry::WlRegistry, ()>
        + Dispatch<wl_callback::WlCallback, u32>
        + HoldsExchange
        + 'static,
{
    /// Connects to `socket` and reads the registry into `state`.
    pub(crate) fn open(
        socket: &Path,
        state: &mut S,
        deadline: Deadline,
    ) -> Result<Self, WaylandFault> {
        Self::over(connect(socket)?, state, deadline)
    }

    /// Reads the registry into `state` over a stream the caller already
    /// connected, so the caller can check the stream's peer first.
    pub(crate) fn over(
        stream: UnixStream,
        state: &mut S,
        deadline: Deadline,
    ) -> Result<Self, WaylandFault> {
        let connection = Connection::from_socket(stream).map_err(|_| WaylandFault::Gone)?;
        let queue = connection.new_event_queue::<S>();
        let registry = connection.display().get_registry(&queue.handle(), ());
        let mut link = Self {
            connection,
            queue,
            registry,
            rounds: 0,
            deadline,
        };
        link.roundtrip(state)?;
        Ok(link)
    }

    /// Waits until the compositor has answered every request sent so far.
    pub(crate) fn roundtrip(&mut self, state: &mut S) -> Result<(), WaylandFault> {
        self.rounds += 1;
        let round = self.rounds;
        let _callback = self.connection.display().sync(&self.queue.handle(), round);
        dispatch_until(
            &mut self.queue,
            state,
            |state| state.exchange().has_finished(round),
            self.deadline,
        )
    }

    /// Dispatches events until `done` holds or the deadline passes, for an
    /// answer that may arrive after the round trip that requested it.
    pub(crate) fn wait(
        &mut self,
        state: &mut S,
        done: impl Fn(&S) -> bool,
    ) -> Result<(), WaylandFault> {
        dispatch_until(&mut self.queue, state, done, self.deadline)
    }

    /// Binds `global` at the lower of its advertised version and `version`.
    pub(crate) fn bind<I, U>(&self, global: &Global, version: u32, data: U) -> I
    where
        I: Proxy + 'static,
        U: Send + Sync + 'static,
        S: Dispatch<I, U>,
    {
        self.registry.bind::<I, U, S>(
            global.name,
            global.version.min(version),
            &self.queue.handle(),
            data,
        )
    }

    /// Sends every queued request without waiting for an answer.
    pub(crate) fn flush(&self) -> Result<(), WaylandFault> {
        self.connection.flush().map_err(|_| WaylandFault::Gone)
    }

    pub(crate) fn handle(&self) -> QueueHandle<S> {
        self.queue.handle()
    }
}

#[cfg(test)]
#[path = "link_tests.rs"]
mod tests;
