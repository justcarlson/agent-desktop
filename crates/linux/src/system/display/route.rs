use agent_desktop_core::AdapterError;

use super::display_server::{DisplayServer, session_server};
pub(crate) use super::wayland_shell::WaylandShell;
use crate::system::display;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Route {
    X11(String),
    Wayland(WaylandShell),
}

/// The route for a display server. A Wayland server asks `shell` which
/// compositor serves it; only Wayland pays for that question.
pub(crate) fn resolve_with(
    server: DisplayServer,
    shell: impl FnOnce() -> Result<WaylandShell, AdapterError>,
) -> Result<Route, AdapterError> {
    match server {
        DisplayServer::X11(display) => Ok(Route::X11(display)),
        DisplayServer::Wayland => shell().map(Route::Wayland),
        DisplayServer::Unavailable(why) => Err(display::unreachable(why)),
    }
}

pub(crate) fn resolve(server: DisplayServer) -> Result<Route, AdapterError> {
    resolve_with(server, crate::system::hyprland::session::session_shell)
}

pub(crate) fn current() -> Result<Route, AdapterError> {
    resolve(session_server())
}

#[cfg(test)]
#[path = "route_tests.rs"]
mod tests;
