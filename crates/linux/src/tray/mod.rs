use agent_desktop_core::{
    ActionRequest, ActionResult, AdapterError, Deadline, ElementState, InteractionLease,
    LiveElement, NativeHandle, ObservedTree, Rect, RefEntry,
};

use crate::system::SharedBus;

mod activate;
mod capability;
mod item_props;
mod menu_event;
mod menu_layout;
mod menu_nodes;
mod menu_resolve;
mod panel_correlate;
mod registration;
mod watcher;
mod world;

#[cfg(test)]
mod live_tests;

/// A tray item or one of its menu entries, named by D-Bus identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrayTarget {
    pub(crate) owner: String,
    pub(crate) object_path: String,
    pub(crate) menu_path: Option<String>,
    pub(crate) entry_id: Option<i32>,
    pub(crate) revision: Option<u32>,
}

pub(crate) fn target_of(_handle: &NativeHandle) -> Option<&TrayTarget> {
    None
}

/// Whether a stored ref names a tray item or one of its menu entries.
pub(crate) fn owns(_entry: &RefEntry) -> bool {
    false
}

#[expect(dead_code, reason = "stub the owning lane replaces")]
pub(crate) fn enrich(
    _tree: &mut ObservedTree,
    _bus: &SharedBus,
    _deadline: Deadline,
) -> Result<(), AdapterError> {
    Ok(())
}

pub(crate) fn resolve(
    _entry: &RefEntry,
    _deadline: Deadline,
) -> Result<NativeHandle, AdapterError> {
    Err(AdapterError::not_supported("resolving a tray item"))
}

pub(crate) fn execute(
    _target: &TrayTarget,
    _request: ActionRequest,
    _lease: &InteractionLease,
) -> Result<ActionResult, AdapterError> {
    Err(AdapterError::not_supported("execute_action on a tray item"))
}

pub(crate) fn live_actions(
    _target: &TrayTarget,
    _deadline: Deadline,
) -> Result<Option<Vec<String>>, AdapterError> {
    Ok(None)
}

pub(crate) fn live_state(
    _target: &TrayTarget,
    _deadline: Deadline,
) -> Result<Option<ElementState>, AdapterError> {
    Ok(None)
}

pub(crate) fn live_element(
    _target: &TrayTarget,
    _deadline: Deadline,
) -> Result<LiveElement, AdapterError> {
    Err(AdapterError::not_supported(
        "get_live_element on a tray item",
    ))
}

pub(crate) fn bounds(
    _target: &TrayTarget,
    _deadline: Deadline,
) -> Result<Option<Rect>, AdapterError> {
    Ok(None)
}
