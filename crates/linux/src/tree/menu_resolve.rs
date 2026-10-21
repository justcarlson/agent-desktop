use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, ErrorCode, LocatorEvidence, NativeHandle, RefEntry,
    SnapshotSurface,
};
use serde_json::json;
use zbus::Connection;

use super::absence::Liveness;
use super::menu_context::MenuContext;
use super::node_evidence::read_evidence;
use super::observe_menu::rooted;
use super::path_descent::{Level, Levels};
use super::resolve::{Source, resolve};
use super::resolve_errors::{ambiguous, settle_gone, stopped_app};
use super::resolve_gate::stored_window;
use super::resolve_search::{Searched, evaluate};
use super::walker::walk_tree;
use super::{AtspiElement, WalkLimits, atspi_element, read_node};
use crate::system::menu_surface::popups_at;
use crate::system::shell::container::MENU;
use crate::system::{SharedBus, owner_process_id, process_facts, within};

pub(crate) fn is_menu_entry(entry: &RefEntry) -> bool {
    entry.source.source_surface == SnapshotSurface::Menu
}

/// Whether a node sits inside a popup menu: the root its walk started from is
/// itself a menu node, which a window frame never is.
pub(crate) fn inside_popup(bus: &SharedBus, handle: &NativeHandle, deadline: Deadline) -> bool {
    let Ok(element) = atspi_element(handle) else {
        return false;
    };
    let Some(frame) = element.frame() else {
        return false;
    };
    let Ok(connection) = bus.get(deadline) else {
        return false;
    };
    within(deadline, async {
        read_node(connection.inner(), frame).await
    })
    .is_ok_and(|node| node.traits.role.present() == Some(&MENU))
}

/// The answer when the popup's own window cannot prove a screen position.
/// Waiting for the same missing geometry would only run out the budget.
pub(crate) fn no_position() -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "The menu item has no usable screen position, so a pointer click cannot be aimed at it",
    )
    .with_suggestion("Keep the menu open and take a fresh menu snapshot before clicking the item")
    .with_details(
        json!({ "check": "geometry", "physical_delivery_started": false, "retryable": false }),
    )
    .with_disposition(DeliverySemantics::not_delivered())
}

fn unanswered() -> AdapterError {
    AdapterError::timeout("The accessible object did not answer in time")
}

pub(crate) fn menu_closed() -> AdapterError {
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
        let mut read = within(self.deadline, read_evidence(connection, element))?;
        read.evidence.identifiers = read
            .evidence
            .identifiers
            .with_retained_object(element.path().to_owned());
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
    let popups = popups_at(bus, &window.frame(), false, deadline)
        .map_err(|error| settle_gone(error, window.pid()))?;
    if popups.is_empty() {
        return Err(menu_closed());
    }
    let mut candidates = Vec::new();
    for popup in popups {
        candidates.push((popup.context, popup.root));
    }
    resolve_contextual(entry, window.frame(), candidates, |popup, scoped_entry| {
        let mut source = MenuSource {
            bus,
            connection: connection.clone(),
            popup,
            levels: None,
            deadline,
        };
        resolve(&mut source, scoped_entry, deadline)
    })
    .map(AtspiElement::into_native_handle)
}

fn resolve_contextual(
    entry: &RefEntry,
    owner: AtspiElement,
    candidates: Vec<(MenuContext, AtspiElement)>,
    mut resolve: impl FnMut(AtspiElement, &RefEntry) -> Result<AtspiElement, AdapterError>,
) -> Result<AtspiElement, AdapterError> {
    let mut landed = Vec::new();
    let mut refusal: Option<AdapterError> = None;
    for (context, popup) in candidates {
        let Some(scoped_entry) = context.entry_for(entry) else {
            continue;
        };
        match resolve(popup, &scoped_entry) {
            Ok(mut element) => {
                element.set_menu_owner(&owner);
                landed.push(element);
            }
            Err(error) if is_stale(&error) => {}
            Err(error) => refusal = refusal.or(Some(error)),
        }
    }
    choose(landed, refusal)
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
