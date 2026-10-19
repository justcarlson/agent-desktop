use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode};
use serde_json::json;

const DETAIL_CAP: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Failure {
    Empty,
    TypeUnavailable,
    NoKeyboard,
    SocketMissing,
    SocketRefused,
    Other(String),
}

/// Sorts what a `wl-clipboard` tool printed into one distinct failure. The
/// connection messages are told apart by their cause, the text after the
/// colon.
pub(crate) fn classify(stderr: &str) -> Failure {
    if stderr.contains("Nothing is copied") {
        Failure::Empty
    } else if stderr.contains("not available as requested type")
        || stderr.contains("No suitable type of content copied")
    {
        Failure::TypeUnavailable
    } else if stderr.contains("This seat has no keyboard") {
        Failure::NoKeyboard
    } else if stderr.contains("Failed to connect to a Wayland server") {
        if stderr.contains("Connection refused") {
            Failure::SocketRefused
        } else if stderr.contains("No such file or directory") {
            Failure::SocketMissing
        } else {
            Failure::Other(stderr.trim().to_owned())
        }
    } else {
        Failure::Other(stderr.trim().to_owned())
    }
}

fn reasoned(code: ErrorCode, message: &str, reason: &str, suggestion: &str) -> AdapterError {
    AdapterError::new(code, message)
        .with_suggestion(suggestion)
        .with_details(json!({ "reason": reason }))
}

fn bounded(text: &str) -> String {
    text.chars().take(DETAIL_CAP).collect()
}

/// The error for a tool that ran and failed. `unknown` carries the delivery
/// of a failure nothing explains: a write that failed for an unknown cause
/// may still have reached the clipboard.
pub(crate) fn failure_error(failure: &Failure, unknown: DeliverySemantics) -> AdapterError {
    let known = DeliverySemantics::not_delivered();
    match failure {
        Failure::Empty | Failure::TypeUnavailable => AdapterError::new(
            ErrorCode::ActionFailed,
            "The Wayland clipboard holds nothing of the requested kind",
        )
        .with_disposition(known),
        Failure::NoKeyboard => reasoned(
            ErrorCode::ActionFailed,
            "The Wayland seat has no keyboard, so the clipboard cannot be used",
            "no_keyboard_seat",
            "Use a headless session that has a virtual keyboard or other input device",
        )
        .with_disposition(known),
        Failure::SocketMissing => reasoned(
            ErrorCode::PlatformNotSupported,
            "The Wayland socket does not exist",
            "socket_missing",
            "Check that WAYLAND_DISPLAY and XDG_RUNTIME_DIR name a running compositor",
        )
        .with_disposition(known),
        Failure::SocketRefused => reasoned(
            ErrorCode::PlatformNotSupported,
            "The Wayland socket refused the connection",
            "socket_refused",
            "The compositor behind WAYLAND_DISPLAY has stopped; start it again",
        )
        .with_disposition(known),
        Failure::Other(text) => {
            AdapterError::new(ErrorCode::ActionFailed, "The Wayland clipboard tool failed")
                .with_platform_detail(bounded(text))
                .with_disposition(unknown)
        }
    }
}

#[cfg(test)]
#[path = "wayland_errors_tests.rs"]
mod tests;
