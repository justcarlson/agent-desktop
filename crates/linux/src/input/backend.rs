#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

use agent_desktop_core::{AdapterError, ErrorCode};
use serde_json::json;

use crate::system::display_server::{DisplayServer, session_server};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Backend {
    X11(String),
    Wayland,
}

pub(crate) fn backend() -> Result<Backend, AdapterError> {
    select(session_server())
}

pub(crate) fn select(server: DisplayServer) -> Result<Backend, AdapterError> {
    match server {
        DisplayServer::X11(display) => Ok(Backend::X11(display)),
        DisplayServer::Wayland => Ok(Backend::Wayland),
        DisplayServer::Unavailable(unavailable) => Err(AdapterError::new(
            ErrorCode::PlatformNotSupported,
            "No display server is reachable: set WAYLAND_DISPLAY or DISPLAY to a running session",
        )
        .with_suggestion("Run from inside a graphical session, or set WAYLAND_DISPLAY or DISPLAY")
        .with_details(json!({
            "cause": unavailable.cause.as_str(),
            "tried": unavailable.detail,
            "physical_delivery_started": false,
        }))
        .with_disposition(agent_desktop_core::DeliverySemantics::not_delivered())),
    }
}

#[cfg(test)]
#[path = "backend_tests.rs"]
mod tests;
