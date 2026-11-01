//! The bound Hyprland instance that pointer input and the cursor guard go
//! to. A unit test runs on a developer's own desktop, so under test there is
//! never a route and no test can send a live event.

use agent_desktop_core::AdapterError;

use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;

/// The bound Hyprland instance when this session routes to one.
#[cfg(not(test))]
pub(crate) fn bound_route() -> Result<Option<HyprlandBinding>, AdapterError> {
    crate::system::hyprland::bind::routed_binding::routed_binding()
}

/// No route under test.
#[cfg(test)]
pub(crate) fn bound_route() -> Result<Option<HyprlandBinding>, AdapterError> {
    Ok(None)
}
