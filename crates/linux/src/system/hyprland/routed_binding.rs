use agent_desktop_core::AdapterError;

use super::hyprland_binding::HyprlandBinding;
use crate::system::display::display_server::{DisplayServer, session_server};
use crate::system::display::route::{self, Route, WaylandShell};

/// The bound Hyprland instance when this session's display route is
/// Hyprland, `None` on any other route. A Hyprland session that was found
/// but refused is an error, as on every other Hyprland route.
pub(crate) fn routed_binding() -> Result<Option<HyprlandBinding>, AdapterError> {
    match session_server() {
        DisplayServer::Wayland => match route::resolve(DisplayServer::Wayland)? {
            Route::Wayland(WaylandShell::Hyprland(binding)) => Ok(Some(binding)),
            _ => Ok(None),
        },
        _ => Ok(None),
    }
}
