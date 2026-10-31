//! A capture's own event queue and registry on an existing connection.
//! Objects created on it dispatch to its state; requests may still name
//! objects of other queues, such as a foreign toplevel handle.

use agent_desktop_core::Deadline;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::{Connection, EventQueue, QueueHandle};

use super::capture_fault::CaptureFault;
use super::capture_state::CaptureState;
use crate::system::wayland::dispatch::dispatch_until;
use crate::system::wayland::wayland_fault::WaylandFault;

/// One capture queue, what it has been told, and its registry.
pub(crate) struct CaptureQueue {
    connection: Connection,
    queue: EventQueue<CaptureState>,
    pub(crate) state: CaptureState,
    pub(crate) registry: WlRegistry,
}

impl CaptureQueue {
    /// Opens a queue on `connection` and waits for the registry's globals.
    pub(crate) fn open(connection: &Connection, deadline: Deadline) -> Result<Self, CaptureFault> {
        let queue = connection.new_event_queue::<CaptureState>();
        let registry = connection.display().get_registry(&queue.handle(), ());
        let mut capture = Self {
            connection: connection.clone(),
            queue,
            state: CaptureState::default(),
            registry,
        };
        capture.sync(deadline).map_err(CaptureFault::Setup)?;
        Ok(capture)
    }

    pub(crate) fn handle(&self) -> QueueHandle<CaptureState> {
        self.queue.handle()
    }

    /// Waits until the compositor has answered every request sent so far.
    pub(crate) fn sync(&mut self, deadline: Deadline) -> Result<(), WaylandFault> {
        let base = self.state.synced;
        let _callback = self.connection.display().sync(&self.queue.handle(), ());
        self.wait(|state| state.synced > base, deadline)
    }

    /// Dispatches this queue's events until `done` holds or the deadline
    /// passes.
    pub(crate) fn wait(
        &mut self,
        done: impl Fn(&CaptureState) -> bool,
        deadline: Deadline,
    ) -> Result<(), WaylandFault> {
        dispatch_until(&mut self.queue, &mut self.state, done, deadline)
    }

    /// Sends every queued request, such as the destructors of a finished
    /// capture. A broken connection has nothing left to release.
    pub(crate) fn flush(&self) {
        let _ = self.connection.flush();
    }
}
