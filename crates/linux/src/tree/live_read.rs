use agent_desktop_core::{
    AdapterError, Deadline, ElementState, ErrorCode, LiveElement, NativeHandle, Rect,
};
use zbus::Connection;

use super::absence::Liveness;
use super::live_offscreen::{global_position, placement};
use super::live_project::{
    live_actions, live_bounds, live_element, live_state, live_value, pointer_rect, settled_by_read,
};
use super::node_evidence::{NodeEvidence, read_evidence};
use super::placement::Placement;
use super::resolve_errors::{app_exited, object_gone, settle_gone, stopped_app, stopped_aware};
use super::{AtspiElement, atspi_element};
use crate::system::{SharedBus, within};

/// What one live read needs from the bus. The bus implements it; a test
/// substitutes in-memory nodes and counts the reads.
pub(crate) trait NodeReader {
    fn read(&mut self, element: &AtspiElement) -> Result<NodeEvidence, AdapterError>;
    fn stopped_app(&mut self, pid: u32) -> Option<String>;
    /// Where the node sits in its window, read once.
    fn placement(&mut self, element: &AtspiElement, deadline: Deadline) -> Placement;
    /// Whether the node's toolkit reports global screen positions.
    fn global_position(&mut self, element: &AtspiElement, deadline: Deadline) -> bool;
}

struct BusReader<'a> {
    bus: &'a SharedBus,
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

    fn placement(&mut self, element: &AtspiElement, deadline: Deadline) -> Placement {
        placement(self.bus, element, deadline)
    }

    fn global_position(&mut self, element: &AtspiElement, deadline: Deadline) -> bool {
        global_position(self.bus, element, deadline)
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
    project: impl FnOnce(&mut BusReader<'_>, &NativeHandle) -> Result<T, AdapterError>,
) -> Result<T, AdapterError> {
    atspi_element(handle)?;
    let connection = bus.get(deadline)?.inner().clone();
    let mut reader = BusReader {
        bus,
        connection: &connection,
        deadline,
    };
    project(&mut reader, handle)
}

pub(crate) fn value_via<R: NodeReader>(
    reader: &mut R,
    handle: &NativeHandle,
) -> Result<Option<String>, AdapterError> {
    live_value(&read_live(reader, handle)?.evidence)
}

pub(crate) fn state_via<R: NodeReader>(
    reader: &mut R,
    handle: &NativeHandle,
    deadline: Deadline,
) -> Result<Option<ElementState>, AdapterError> {
    let node = read_live(reader, handle)?;
    let placed = if settled_by_read(&node.read) {
        Placement::default()
    } else {
        reader.placement(atspi_element(handle)?, deadline)
    };
    live_state(&node.read, &node.evidence, &placed).map(Some)
}

pub(crate) fn actions_via<R: NodeReader>(
    reader: &mut R,
    handle: &NativeHandle,
) -> Result<Option<Vec<String>>, AdapterError> {
    live_actions(&read_live(reader, handle)?.evidence).map(Some)
}

/// The one placement a live element needs, read only when the state or the
/// bounds want it: the state wants it unless the node read already settles
/// offscreen, the bounds want it unless the toolkit's extents are global.
pub(crate) fn element_via<R: NodeReader>(
    reader: &mut R,
    handle: &NativeHandle,
    deadline: Deadline,
) -> Result<LiveElement, AdapterError> {
    let node = read_live(reader, handle)?;
    let element = atspi_element(handle)?;
    let global = reader.global_position(element, deadline);
    let placed = if global && settled_by_read(&node.read) {
        Placement::default()
    } else {
        reader.placement(element, deadline)
    };
    live_element(&node.read, &node.evidence, &placed, global)
}

pub(crate) fn bounds_via<R: NodeReader>(
    reader: &mut R,
    handle: &NativeHandle,
    deadline: Deadline,
) -> Result<Option<Rect>, AdapterError> {
    let node = read_live(reader, handle)?;
    let element = atspi_element(handle)?;
    if reader.global_position(element, deadline) {
        return Ok(live_bounds(&node.read));
    }
    let placed = reader.placement(element, deadline);
    Ok(pointer_rect(&node.read, &placed))
}

pub(crate) fn get_live_value(
    bus: &SharedBus,
    handle: &NativeHandle,
    deadline: Deadline,
) -> Result<Option<String>, AdapterError> {
    with_reader(bus, handle, deadline, |reader, handle| {
        value_via(reader, handle)
    })
}

pub(crate) fn get_live_state(
    bus: &SharedBus,
    handle: &NativeHandle,
    deadline: Deadline,
) -> Result<Option<ElementState>, AdapterError> {
    with_reader(bus, handle, deadline, |reader, handle| {
        state_via(reader, handle, deadline)
    })
}

pub(crate) fn get_live_actions(
    bus: &SharedBus,
    handle: &NativeHandle,
    deadline: Deadline,
) -> Result<Option<Vec<String>>, AdapterError> {
    with_reader(bus, handle, deadline, |reader, handle| {
        actions_via(reader, handle)
    })
}

pub(crate) fn get_live_element(
    bus: &SharedBus,
    handle: &NativeHandle,
    deadline: Deadline,
) -> Result<LiveElement, AdapterError> {
    with_reader(bus, handle, deadline, |reader, handle| {
        element_via(reader, handle, deadline)
    })
}

pub(crate) fn get_element_bounds(
    bus: &SharedBus,
    handle: &NativeHandle,
    deadline: Deadline,
) -> Result<Option<Rect>, AdapterError> {
    with_reader(bus, handle, deadline, |reader, handle| {
        bounds_via(reader, handle, deadline)
    })
}

#[cfg(test)]
#[path = "live_read_tests.rs"]
mod tests;
