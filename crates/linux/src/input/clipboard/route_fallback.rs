use std::ffi::OsString;

use super::wayland::locate;
use crate::system::route::Route;

pub(crate) const COPY: &str = "wl-copy";
pub(crate) const PASTE: &str = "wl-paste";

/// What decides whether a Wayland session may serve the clipboard itself: the
/// search path the Wayland tools are looked up in, and the X11 display an
/// Xwayland bridge offers when they are missing.
#[derive(Debug, Clone, Default)]
pub(crate) struct Fallback {
    pub(crate) search_path: Option<OsString>,
    pub(crate) display: Option<String>,
}

impl Fallback {
    pub(crate) fn from_process() -> Self {
        Self {
            search_path: std::env::var_os("PATH"),
            display: std::env::var("DISPLAY")
                .ok()
                .filter(|display| !display.is_empty()),
        }
    }

    /// The route a call runs on. A Wayland session whose tools for this call
    /// are missing uses the X11 route when a display is set, because an
    /// Xwayland client reads and writes the same clipboard. With no display
    /// the Wayland route stays, and it names the missing package.
    pub(crate) fn route(&self, route: Route, tools: &[&str]) -> Route {
        match (&route, &self.display) {
            (Route::Wayland, Some(display))
                if tools
                    .iter()
                    .any(|tool| locate(tool, self.search_path.as_deref()).is_none()) =>
            {
                Route::X11(display.clone())
            }
            _ => route,
        }
    }
}

#[cfg(test)]
#[path = "route_fallback_tests.rs"]
mod tests;
