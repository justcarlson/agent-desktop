use agent_desktop_core::{
    AdapterError, Deadline, ErrorCode, LocatorEvidence, NativeHandle, RefEntry, SnapshotSurface,
};
use serde_json::json;
use zbus::Connection;

use super::absence::Liveness;
use super::node_evidence::read_evidence;
use super::observe_menu::rooted;
use super::path_descent::{Level, Levels};
use super::resolve::{Source, resolve};
use super::resolve_errors::{ambiguous, settle_gone, stopped_app};
use super::resolve_gate::stored_window;
use super::resolve_search::{Searched, evaluate};
use super::walker::walk_tree;
use super::{AtspiElement, WalkLimits};
use crate::system::menu_surface::popups_at;
use crate::system::{SharedBus, owner_process_id, process_facts, within};

pub(crate) fn is_menu_entry(entry: &RefEntry) -> bool {
    entry.source.source_surface == SnapshotSurface::Menu
}

fn unanswered() -> AdapterError {
    AdapterError::timeout("The accessible object did not answer in time")
}

fn menu_closed() -> AdapterError {
    AdapterError::stale_ref_because("The menu the ref was taken in is no longer open").with_details(
        json!({
            "kind": "menu_closed",
            "complete": true,
            "retryable": false,
        }),
    )
}

struct MenuSource<'a> {
    bus: &'a SharedBus,
    connection: Connection,
    popup: AtspiElement,
    levels: Option<Levels>,
    deadline: Deadline,
}

impl Source for MenuSource<'_> {
    fn open_window(&mut self, _entry: &RefEntry) -> Result<AtspiElement, AdapterError> {
        let root = rooted(&self.popup);
        self.levels = Some(Levels::new(self.connection.clone(), &root, self.deadline));
        Ok(root)
    }

    fn level(&mut self, element: &AtspiElement) -> Result<Level<AtspiElement>, AdapterError> {
        match self.levels.as_mut() {
            Some(levels) => levels.read(element),
            None => Err(AdapterError::internal(
                "A level was read before the popup was opened",
            )),
        }
    }

    fn evidence(&mut self, element: &AtspiElement) -> Result<LocatorEvidence, AdapterError> {
        let connection = &self.connection;
        let read = within(self.deadline, read_evidence(connection, element))?;
        match read.liveness {
            Liveness::TimedOut => Err(unanswered()),
            _ => Ok(read.evidence),
        }
    }

    fn search(&mut self, frame: &AtspiElement, entry: &RefEntry) -> Result<Searched, AdapterError> {
        let walk = walk_tree(self.bus, frame, &WalkLimits::default(), self.deadline)?;
        Ok(evaluate(&walk, entry, false))
    }

    fn stopped_app(&mut self, pid: u32) -> Option<String> {
        stopped_app(pid)
    }
}

fn is_stale(error: &AdapterError) -> bool {
    error.code == ErrorCode::StaleRef
}

/// Resolves a ref taken in a menu: the popup is found again from the window
/// the ref names, because a popup of GTK 3 and of Chromium is a new object on
/// every open, and the stored path descends from that popup. A ref that no
/// open popup answers to is stale, a closed menu most often; one that two
/// open popups answer to is ambiguous.
pub(crate) fn resolve_menu(
    bus: &SharedBus,
    entry: &RefEntry,
    deadline: Deadline,
) -> Result<NativeHandle, AdapterError> {
    let window = stored_window(entry)?;
    window.check_process(&process_facts::lookup(window.pid()), entry)?;
    let connection = bus.get(deadline)?.inner().clone();
    within(deadline, async {
        let owner = owner_process_id(&connection, window.bus()).await;
        window.check_owner(owner, entry)
    })?;
    let popups = popups_at(bus, &window.frame(), false, deadline).map_err(settle_gone)?;
    if popups.is_empty() {
        return Err(menu_closed());
    }
    let mut landed = Vec::new();
    let mut refusal: Option<AdapterError> = None;
    for popup in popups {
        let mut source = MenuSource {
            bus,
            connection: connection.clone(),
            popup: popup.root,
            levels: None,
            deadline,
        };
        match resolve(&mut source, entry, deadline) {
            Ok(element) => landed.push(element),
            Err(error) if is_stale(&error) => {}
            Err(error) => refusal = refusal.or(Some(error)),
        }
    }
    choose(landed, refusal).map(AtspiElement::into_native_handle)
}

fn choose(
    mut landed: Vec<AtspiElement>,
    refusal: Option<AdapterError>,
) -> Result<AtspiElement, AdapterError> {
    match (landed.len(), refusal) {
        (1, _) => landed.pop().ok_or_else(menu_closed),
        (0, Some(error)) => Err(error),
        (0, None) => Err(menu_closed()),
        (count, _) => Err(ambiguous(count)),
    }
}

#[cfg(test)]
#[path = "menu_resolve_tests.rs"]
mod tests;
