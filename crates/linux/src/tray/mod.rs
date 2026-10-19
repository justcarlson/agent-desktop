#![expect(
    dead_code,
    reason = "the shell observer that calls read and the view is not on this branch"
)]

use agent_desktop_core::{
    ActionRequest, ActionResult, AdapterError, Deadline, ElementState, InteractionLease,
    LiveElement, NativeHandle, Rect, RefEntry,
};

mod activate;
mod call;
mod capability;
mod execute;
mod identity;
mod introspect;
mod item_props;
mod live;
mod menu_event;
mod menu_layout;
mod menu_nodes;
mod menu_resolve;
mod node_evidence;
mod panel_correlate;
mod registration;
mod resolve;
pub(crate) mod view;
mod watcher;
mod world;

#[cfg(test)]
mod fit_tests;
#[cfg(test)]
mod live_tests;

pub(crate) fn read(deadline: Deadline) -> Result<view::TrayView, AdapterError> {
    view::read(deadline)
}

/// A tray item or one of its menu entries, named by D-Bus identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrayTarget {
    pub(crate) owner: String,
    pub(crate) object_path: String,
    pub(crate) menu_path: Option<String>,
    pub(crate) entry_id: Option<i32>,
    pub(crate) revision: Option<u32>,
    pub(crate) label_path: Vec<String>,
}

pub(crate) fn target_of(handle: &NativeHandle) -> Option<&TrayTarget> {
    handle.downcast_ref::<TrayTarget>()
}

/// Whether a stored ref names a tray item or one of its menu entries.
pub(crate) fn owns(entry: &RefEntry) -> bool {
    entry
        .identity
        .native_id
        .as_ref()
        .is_some_and(|id| identity::is_tray_id(&id.value))
}

pub(crate) fn resolve(entry: &RefEntry, deadline: Deadline) -> Result<NativeHandle, AdapterError> {
    resolve::resolve(entry, deadline)
}

pub(crate) fn execute(
    target: &TrayTarget,
    request: ActionRequest,
    lease: &InteractionLease,
) -> Result<ActionResult, AdapterError> {
    execute::execute(target, request, lease)
}

pub(crate) fn live_actions(
    target: &TrayTarget,
    deadline: Deadline,
) -> Result<Option<Vec<String>>, AdapterError> {
    live::actions(target, deadline).map(Some)
}

pub(crate) fn live_state(
    target: &TrayTarget,
    deadline: Deadline,
) -> Result<Option<ElementState>, AdapterError> {
    live::state(target, deadline).map(Some)
}

pub(crate) fn live_element(
    target: &TrayTarget,
    deadline: Deadline,
) -> Result<LiveElement, AdapterError> {
    live::element(target, deadline)
}

pub(crate) fn bounds(
    _target: &TrayTarget,
    _deadline: Deadline,
) -> Result<Option<Rect>, AdapterError> {
    Ok(None)
}
