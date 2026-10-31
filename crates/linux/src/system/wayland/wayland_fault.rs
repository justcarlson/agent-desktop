use std::path::Path;

use agent_desktop_core::AdapterError;

use crate::system::display::display_server::{DisplayCause, display_cause_error};

/// Why a Wayland exchange ended without an answer: the compositor closed the
/// connection, it did not answer before the deadline, or it ended the
/// connection with a protocol error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WaylandFault {
    Gone,
    Silent,
    Protocol(String),
}

impl WaylandFault {
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
        }
    }
}

#[cfg(test)]
#[path = "wayland_fault_tests.rs"]
mod tests;
