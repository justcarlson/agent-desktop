//! The bound Hyprland instance that pointer input and the cursor guard go
//! to. A unit test runs on a developer's own desktop, so under test there is
//! never a live route and no test can send a live event. A test that only
//! asks whether a route exists sets [`TEST_ROUTE_VARIABLE`] in a child
//! process and gets a binding to sockets that do not exist.

use agent_desktop_core::AdapterError;

use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;

/// The bound Hyprland instance when this session routes to one.
#[cfg(not(test))]
pub(crate) fn bound_route() -> Result<Option<HyprlandBinding>, AdapterError> {
    crate::system::hyprland::bind::routed_binding::routed_binding()
}

/// Names the variable that gives a test process an unreachable binding.
#[cfg(test)]
pub(crate) const TEST_ROUTE_VARIABLE: &str = "AD_TEST_HYPRLAND_ROUTE";

/// No live route under test: an unreachable binding when
/// [`TEST_ROUTE_VARIABLE`] is set, else none.
#[cfg(test)]
pub(crate) fn bound_route() -> Result<Option<HyprlandBinding>, AdapterError> {
    Ok(
        std::env::var_os(TEST_ROUTE_VARIABLE).map(|_| HyprlandBinding {
            signature: "test".to_owned(),
            ipc_socket: std::path::PathBuf::from("/nonexistent/.socket.sock"),
            wayland_socket: std::path::PathBuf::from("/nonexistent/wayland-test"),
            pid: 0,
            start_time: 0,
        }),
    )
}
