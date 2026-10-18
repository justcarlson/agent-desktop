use agent_desktop_core::{
    AdapterError, Deadline, ElementState, ErrorCode, LiveElement, NativeHandle, Rect,
};
use zbus::Connection;

use super::absence::Liveness;
use super::live_project::{live_actions, live_bounds, live_element, live_state, live_value};
use super::node_evidence::{NodeEvidence, read_evidence};
use super::resolve_errors::{app_exited, object_gone, settle_gone, stopped_app, stopped_aware};
use super::{AtspiElement, atspi_element};
use crate::system::{SharedBus, within};

/// What one live read needs from the bus. The bus implements it; a test
/// substitutes in-memory nodes and counts the reads.
pub(crate) trait NodeReader {
    fn read(&mut self, element: &AtspiElement) -> Result<NodeEvidence, AdapterError>;
    fn stopped_app(&mut self, pid: u32) -> Option<String>;
}

struct BusReader<'a> {
    connection: &'a Connection,
    deadline: Deadline,
}

impl NodeReader for BusReader<'_> {
    fn read(&mut self, element: &AtspiElement) -> Result<NodeEvidence, AdapterError> {
        within(self.deadline, read_evidence(self.connection, element))
    }

    fn stopped_app(&mut self, pid: u32) -> Option<String> {
        stopped_app(pid)
    }
}

fn unanswered() -> AdapterError {
    AdapterError::timeout("The accessible object did not answer in time")
}

/// Reads the node a handle names once and maps what the read shows. An
/// application that left and a destroyed object are a settled `STALE_REF`; a
/// node that did not answer is a timeout, `APP_UNRESPONSIVE` when `/proc`
/// shows its process stopped.
pub(crate) fn read_live<R: NodeReader>(
    reader: &mut R,
    handle: &NativeHandle,
) -> Result<NodeEvidence, AdapterError> {
    let element = atspi_element(handle)?;
    let pid = element.frame().map_or_else(|| element.pid(), |f| f.pid());
    let outcome = reader.read(element).and_then(|node| match node.liveness {
        Liveness::Alive => Ok(node),
        Liveness::TimedOut => Err(unanswered()),
        Liveness::ObjectGone => Err(object_gone()),
        Liveness::AppGone => Err(app_exited()),
    });
    outcome.map_err(|error| {
        let error = settle_gone(error);
        if error.code == ErrorCode::Timeout {
            let stopped = reader.stopped_app(pid);
            stopped_aware(error, stopped.as_deref())
        } else {
            error
        }
    })
}

fn with_reader<T>(
    bus: &SharedBus,
    handle: &NativeHandle,
    deadline: Deadline,
    project: impl FnOnce(&NodeEvidence) -> Result<T, AdapterError>,
) -> Result<T, AdapterError> {
    let connection = bus.get(deadline)?.inner().clone();
    let mut reader = BusReader {
        connection: &connection,
        deadline,
    };
    project(&read_live(&mut reader, handle)?)
}

pub(crate) fn get_live_value(
    bus: &SharedBus,
    handle: &NativeHandle,
    deadline: Deadline,
) -> Result<Option<String>, AdapterError> {
    with_reader(bus, handle, deadline, |node| live_value(&node.evidence))
}

pub(crate) fn get_live_state(
    bus: &SharedBus,
    handle: &NativeHandle,
    deadline: Deadline,
) -> Result<Option<ElementState>, AdapterError> {
    with_reader(bus, handle, deadline, |node| {
        live_state(&node.read, &node.evidence).map(Some)
    })
}

pub(crate) fn get_live_actions(
    bus: &SharedBus,
    handle: &NativeHandle,
    deadline: Deadline,
) -> Result<Option<Vec<String>>, AdapterError> {
    with_reader(bus, handle, deadline, |node| {
        live_actions(&node.evidence).map(Some)
    })
}

pub(crate) fn get_live_element(
    bus: &SharedBus,
    handle: &NativeHandle,
    deadline: Deadline,
) -> Result<LiveElement, AdapterError> {
    with_reader(bus, handle, deadline, |node| {
        live_element(&node.read, &node.evidence)
    })
}

pub(crate) fn get_element_bounds(
    bus: &SharedBus,
    handle: &NativeHandle,
    deadline: Deadline,
) -> Result<Option<Rect>, AdapterError> {
    with_reader(bus, handle, deadline, |node| Ok(live_bounds(&node.read)))
}

#[cfg(test)]
#[path = "live_read_tests.rs"]
mod tests;
