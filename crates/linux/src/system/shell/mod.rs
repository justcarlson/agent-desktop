use agent_desktop_core::{
    AdapterError, Deadline, InteractionLease, InteractionPolicy, ObservationRequest, ObservedTree,
    SnapshotSurface, WindowInfo,
};

use crate::system::SharedBus;
use crate::tree::AtspiElement;

pub(crate) mod container;
pub(crate) mod detect;
pub(crate) mod hit;
pub(crate) mod identity;
pub(crate) mod kinds;
pub(crate) mod live_state;
pub(crate) mod open;
pub(crate) mod window_gate;

#[cfg(test)]
mod live_tests;

/// The shell kinds this session can present, as snapshot surfaces.
pub(crate) fn supported(_bus: &SharedBus) -> Vec<SnapshotSurface> {
    Vec::new()
}

/// Whether the element belongs to the desktop shell rather than to an application.
pub(crate) fn is_shell_element(_element: &AtspiElement) -> bool {
    false
}

/// Whether the window is a shell content window.
pub(crate) fn is_shell_window(_window: &WindowInfo) -> bool {
    false
}

pub(crate) fn resolve_surface(
    _bus: &SharedBus,
    _surface: SnapshotSurface,
    _deadline: Deadline,
) -> Result<WindowInfo, AdapterError> {
    Err(AdapterError::not_supported("resolve_shell_surface"))
}

pub(crate) fn open_surface(
    _bus: &SharedBus,
    _surface: SnapshotSurface,
    _policy: InteractionPolicy,
    _lease: &InteractionLease,
) -> Result<WindowInfo, AdapterError> {
    Err(AdapterError::not_supported("open_system_surface"))
}

pub(crate) fn observe_shell(
    _bus: &SharedBus,
    _window: &WindowInfo,
    _request: &ObservationRequest,
) -> Result<ObservedTree, AdapterError> {
    Err(AdapterError::not_supported("observe_tree for this surface"))
}
