use agent_desktop_core::{
    AdapterError, Deadline, InteractionLease, InteractionPolicy, ObservationRequest,
    ObservationRoot, ObservedTree, SnapshotSurface, WindowInfo,
};

use crate::system::{SharedBus, window_id};
use crate::tree::AtspiElement;

pub(crate) mod container;
pub(crate) mod detect;
pub(crate) mod hit;
pub(crate) mod identity;
pub(crate) mod kinds;
pub(crate) mod light;
pub(crate) mod live_state;
pub(crate) mod open;
pub(crate) mod search;
pub(crate) mod tray_decorator;
pub(crate) mod window_gate;

#[cfg(test)]
mod fake_tree;
#[cfg(test)]
mod live_tests;
#[cfg(test)]
mod live_wayland_tests;

const SUPPORT_BUDGET_MS: u64 = 3_000;

/// The shell kinds this session can present, as snapshot surfaces.
pub(crate) fn supported(bus: &SharedBus) -> Vec<SnapshotSurface> {
    let Ok(deadline) = Deadline::after(SUPPORT_BUDGET_MS) else {
        return Vec::new();
    };
    kinds::advertised(&detect::detect(bus, deadline))
}

/// Whether the element belongs to the desktop shell rather than to an application.
pub(crate) fn is_shell_element(element: &AtspiElement) -> bool {
    identity::is_shell_owner(element.bus_name(), element.pid())
}

/// Whether the window is a shell content window.
pub(crate) fn is_shell_window(window: &WindowInfo) -> bool {
    window_id::decode(&window.id)
        .is_ok_and(|(bus_name, _)| identity::is_shell_owner(&bus_name, window.pid.get()))
}

#[cfg(test)]
pub(super) fn identity_remember_for_tests(root: &identity::ShellRoot) {
    identity::remember(root);
}

pub(crate) fn resolve_surface(
    bus: &SharedBus,
    surface: SnapshotSurface,
    deadline: Deadline,
) -> Result<WindowInfo, AdapterError> {
    let Some(kind) = kinds::kind_of(surface) else {
        return Err(AdapterError::not_supported("resolve_shell_surface"));
    };
    detect::require(&detect::detect(bus, deadline), detect::Need::Tree)?;
    let root = identity::find_root(bus, deadline)?;
    let landmark = open::container_landmark(kind);
    let located =
        container::locate_anchor(bus, &root, container::Anchor::Landmark(landmark), deadline)?;
    open::window_info(kind, &root, &located.container)
}

pub(crate) fn open_surface(
    bus: &SharedBus,
    surface: SnapshotSurface,
    policy: InteractionPolicy,
    lease: &InteractionLease,
) -> Result<WindowInfo, AdapterError> {
    let Some(kind) = kinds::kind_of(surface) else {
        return Err(AdapterError::not_supported("open_system_surface"));
    };
    open::open(bus, kind, policy, lease).map(|(window, _)| window)
}

pub(crate) fn observe_shell(
    bus: &SharedBus,
    window: &WindowInfo,
    request: &ObservationRequest,
) -> Result<ObservedTree, AdapterError> {
    window_gate::verify(bus, &window.id, request.deadline)?;
    let mut inner = *request;
    inner.surface = SnapshotSurface::Window;
    let root = ObservationRoot::Window(window);
    if kinds::kind_of(request.surface) == Some(kinds::ShellKind::Tray) {
        let view = crate::tray::view::read(request.deadline).ok();
        let mut decorator = tray_decorator::TrayDecorator::new(view);
        return crate::tree::observe_tree_with(bus, root, &inner, &mut decorator);
    }
    crate::tree::observe_tree(bus, root, &inner)
}
