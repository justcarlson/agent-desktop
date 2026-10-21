use agent_desktop_core::{
    AdapterError, Deadline, LocatorEvidence, NativeHandle, RefEntry, SnapshotSurface,
};
use zbus::Connection;

use super::absence::Liveness;
use super::menu_resolve::resolve_menu;
use super::node_evidence::read_evidence_placed;
use super::node_position::place_nodes;
use super::node_read::read_node_with_liveness;
use super::observe::is_web_process;
use super::path_descent::{Level, Levels};
use super::resolve::{Source, resolve};
use super::resolve_anchor::resolve_anchor;
use super::resolve_errors::{settle_gone, stopped_app};
use super::resolve_gate::stored_window;
use super::resolve_search::{Searched, evaluate};
use super::walker::walk_tree;
use super::{AtspiElement, WalkLimits};
use crate::system::{SharedBus, owner_process_id, process_facts, shell, within};

fn unanswered() -> AdapterError {
    AdapterError::timeout("The accessible object did not answer in time")
}

/// Reads a resolution from the shared accessibility connection, every call
/// racing the caller's deadline.
pub(crate) struct BusSource<'a> {
    bus: &'a SharedBus,
    connection: Connection,
    levels: Option<Levels>,
    deadline: Deadline,
}

impl<'a> BusSource<'a> {
    pub(crate) fn open(bus: &'a SharedBus, deadline: Deadline) -> Result<Self, AdapterError> {
        crate::system::display_server::require(deadline)?;
        let connection = bus.get(deadline)?.inner().clone();
        Ok(Self {
            bus,
            connection,
            levels: None,
            deadline,
        })
    }
}

impl Source for BusSource<'_> {
    fn open_window(&mut self, entry: &RefEntry) -> Result<AtspiElement, AdapterError> {
        let window = stored_window(entry)?;
        window.check_process(&process_facts::lookup(window.pid()), entry)?;
        let frame = window.frame();
        if shell::is_shell_element(&frame) {
            if let Some(id) = entry.source.source_window_id.as_deref() {
                shell::window_gate::verify(self.bus, id, self.deadline)?;
            }
        }
        let connection = &self.connection;
        within(self.deadline, async {
            let (_, liveness) = read_node_with_liveness(connection, &frame)
                .await
                .map_err(settle_gone)?;
            if liveness == Liveness::TimedOut {
                return Err(unanswered());
            }
            let owner = owner_process_id(connection, window.bus()).await;
            window.check_owner(owner, entry)
        })?;
        self.levels = Some(Levels::new(connection.clone(), &frame, self.deadline));
        Ok(frame)
    }

    fn level(&mut self, element: &AtspiElement) -> Result<Level<AtspiElement>, AdapterError> {
        match self.levels.as_mut() {
            Some(levels) => levels.read(element),
            None => Err(AdapterError::internal(
                "A level was read before the window was opened",
            )),
        }
    }

    fn evidence(&mut self, element: &AtspiElement) -> Result<LocatorEvidence, AdapterError> {
        let connection = &self.connection;
        let read = within(
            self.deadline,
            read_evidence_placed(self.bus, connection, element, self.deadline),
        )?;
        match read.liveness {
            Liveness::TimedOut => Err(unanswered()),
            _ => Ok(read.evidence),
        }
    }

    fn search(&mut self, frame: &AtspiElement, entry: &RefEntry) -> Result<Searched, AdapterError> {
        let mut walk = walk_tree(self.bus, frame, &WalkLimits::default(), self.deadline)?;
        place_nodes(self.bus, &mut walk, self.deadline);
        Ok(evaluate(&walk, entry, is_web_process(frame.pid())))
    }

    fn stopped_app(&mut self, pid: u32) -> Option<String> {
        stopped_app(pid)
    }
}

/// Resolves a stored ref against the live accessibility tree. The element
/// comes back as a native handle that carries the window frame and web
/// context it was found under.
pub(crate) fn resolve_element_strict(
    bus: &SharedBus,
    entry: &RefEntry,
    deadline: Deadline,
) -> Result<NativeHandle, AdapterError> {
    if entry.source.source_surface == SnapshotSurface::Menu {
        return resolve_menu(bus, entry, deadline);
    }
    let mut source = BusSource::open(bus, deadline)?;
    resolve(&mut source, entry, deadline).map(AtspiElement::into_native_handle)
}

/// Lands the element a locator match names by its stored path, gated by the
/// window it was taken in. The handle carries the same ancestry a strict
/// resolution gives, so the observation of the element seeds from it.
pub(crate) fn resolve_locator_anchor(
    bus: &SharedBus,
    entry: &RefEntry,
    deadline: Deadline,
) -> Result<NativeHandle, AdapterError> {
    let mut source = BusSource::open(bus, deadline)?;
    resolve_anchor(&mut source, entry, deadline).map(AtspiElement::into_native_handle)
}

#[cfg(test)]
#[path = "resolve_source_tests.rs"]
mod tests;
