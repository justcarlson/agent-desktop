use std::path::Path;

use agent_desktop_core::{AdapterError, ErrorCode};
use wayland_client::ConnectError;

use crate::system::display::display_server::{DisplayCause, display_cause_error};

/// The client library the `wayland-dlopen` build loads at run time.
pub(crate) const CLIENT_LIBRARY: &str = "libwayland-client.so.0";

/// Why a Wayland exchange ended without an answer: the compositor closed the
/// connection, it did not answer before the deadline, it ended the connection
/// with a protocol error, or the client library could not be loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WaylandFault {
    Gone,
    Silent,
    Protocol(String),
    NoLibrary,
}

impl WaylandFault {
    /// The fault for a connection the client could not start. Only the
    /// `wayland-dlopen` build reports a missing library; the pure-Rust
    /// backend always starts.
    pub(crate) fn from_connect(error: ConnectError) -> Self {
        match error {
            ConnectError::NoWaylandLib => Self::NoLibrary,
            ConnectError::NoCompositor | ConnectError::InvalidFd => Self::Gone,
        }
    }

    /// The error a command reports for this fault on the compositor behind
    /// `socket`. A closed connection is a compositor that is no longer
    /// running, silence is a retryable timeout that names the socket, and a
    /// protocol error is internal because the client and compositor disagree.
    pub(crate) fn into_error(self, socket: &Path) -> AdapterError {
        let place = socket.display().to_string();
        match self {
            Self::Gone => display_cause_error(&DisplayCause::Missing {
                address: format!("{place} closed the connection"),
            }),
            Self::Silent => {
                let mut error = display_cause_error(&DisplayCause::Silent);
                let mut details = error
                    .details
                    .take()
                    .unwrap_or_else(|| serde_json::json!({}));
                details["detail"] = place.into();
                error.with_details(details)
            }
            Self::Protocol(detail) => {
                AdapterError::internal("The Wayland compositor ended the connection with an error")
                    .with_suggestion("Retry; report the platform detail if it repeats")
                    .with_platform_detail(detail)
            }
            Self::NoLibrary => AdapterError::new(
                ErrorCode::PlatformNotSupported,
                "The Wayland client library could not be loaded",
            )
            .with_suggestion("Install libwayland-client, or use a build without wayland-dlopen")
            .with_platform_detail(format!("{CLIENT_LIBRARY} could not be loaded for {place}")),
        }
    }
}

#[cfg(test)]
#[path = "wayland_fault_tests.rs"]
mod tests;
