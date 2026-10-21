use agent_desktop_core::{
    AdapterError, Deadline, ErrorCode, InteractionLease, InteractionPolicy, ObservationRequest,
    ObservationRoot, ObservedTree, SnapshotSurface, WindowInfo,
};

use crate::system::{SharedBus, window_id};
use crate::tree::AtspiElement;

pub(crate) mod container;
pub(crate) mod deliver;
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
#[cfg(test)]
mod outage_live_tests;
#[cfg(test)]
#[path = "tray_observation_tests.rs"]
mod tray_observation_tests;

const SUPPORT_BUDGET_MS: u64 = 3_000;

/// The shell kinds this session can present, as snapshot surfaces.
pub(crate) fn supported(bus: &SharedBus) -> Vec<SnapshotSurface> {
    let Ok(deadline) = Deadline::after(SUPPORT_BUDGET_MS) else {
        return Vec::new();
    };
    kinds::advertised(&detect::detect(bus, deadline))
}

/// The answer for a gnome-shell process that this session's accessibility
/// registry does not list: it belongs to another session, so it is not this
/// session's shell.
pub(crate) fn not_this_sessions_shell() -> AdapterError {
    identity::tree_absent("the shell process is not on this session's accessibility bus")
}

pub(crate) fn is_shell_comm(comm: &str) -> bool {
    identity::is_shell_comm(comm)
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
    if !kinds::is_presented(kind) {
        return Err(AdapterError::new(
            ErrorCode::PlatformNotSupported,
            "The taskbar surface is not offered: no stock shell shows a dock",
        )
        .with_suggestion("Open system-tray, action-center or quick-settings instead"));
    }
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
        let view = tray_observation::read(
            || detect::require_watcher(request.deadline),
            || crate::tray::view::read(request.deadline),
        )?;
        let mut decorator = tray_decorator::TrayDecorator::new(Some(view));
        return crate::tree::observe_tree_with(bus, root, &inner, &mut decorator);
    }
    crate::tree::observe_tree(bus, root, &inner)
}

mod tray_observation {
    use agent_desktop_core::AdapterError;

    use crate::tray::view::TrayView;

    pub(super) fn read(
        require_watcher: impl FnOnce() -> Result<(), AdapterError>,
        read: impl FnOnce() -> Result<TrayView, AdapterError>,
    ) -> Result<TrayView, AdapterError> {
        require_watcher()?;
        read()
    }
}
