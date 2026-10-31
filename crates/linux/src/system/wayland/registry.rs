//! One registry round trip: the globals a compositor advertises, read on a
//! connection to an explicit socket within a deadline.

use std::os::unix::net::UnixStream;

use agent_desktop_core::Deadline;
use wayland_client::protocol::{wl_callback, wl_registry};
use wayland_client::{Connection, Dispatch, QueueHandle};

use super::dispatch::dispatch_until;
use super::wayland_fault::WaylandFault;

#[derive(Default)]
struct Globals {
    interfaces: Vec<String>,
    synced: bool,
}

impl Dispatch<wl_registry::WlRegistry, ()> for Globals {
    fn event(
        state: &mut Self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global { interface, .. } = event {
            state.interfaces.push(interface);
        }
    }
}

impl Dispatch<wl_callback::WlCallback, ()> for Globals {
    fn event(
        state: &mut Self,
        _: &wl_callback::WlCallback,
        event: wl_callback::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = event {
            state.synced = true;
        }
    }
}

/// The interface names of every global the compositor behind `stream`
/// advertises. The connection is used once and closed.
pub(crate) fn global_interfaces(
    stream: UnixStream,
    deadline: Deadline,
) -> Result<Vec<String>, WaylandFault> {
    let connection = Connection::from_socket(stream).map_err(|_| WaylandFault::Gone)?;
    let mut queue = connection.new_event_queue::<Globals>();
    let handle = queue.handle();
    let display = connection.display();
    let _registry = display.get_registry(&handle, ());
    let _callback = display.sync(&handle, ());
    let mut globals = Globals::default();
    dispatch_until(&mut queue, &mut globals, |state| state.synced, deadline)?;
    Ok(globals.interfaces)
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
