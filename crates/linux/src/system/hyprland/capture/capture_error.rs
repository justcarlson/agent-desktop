//! The error each capture fault maps to, and the refusal for a compositor
//! that enforces permissions. No error carries pixels from an earlier frame.

use std::path::Path;

use agent_desktop_core::{AdapterError, ErrorCode};
use serde_json::json;

use super::enforce_option::ENFORCE_OPTION;
use crate::system::display::display_server::{DisplayCause, display_cause_error};
use crate::system::wayland::capture::capture_fault::CaptureFault;
use crate::system::wayland::capture::frame_failure::FrameFailure;

/// The refusal sent before any capture object exists, because Hyprland
/// would withhold the frame without telling anyone.
pub(crate) fn permission_refusal() -> AdapterError {
    AdapterError::new(
        ErrorCode::PermDenied,
        format!(
            "Hyprland enforces permissions ({ENFORCE_OPTION}) and withholds screen capture from this tool without asking"
        ),
    )
    .with_suggestion(format!(
        "Set {ENFORCE_OPTION} to false in the Hyprland config, then retry"
    ))
    .with_details(json!({ "option": ENFORCE_OPTION, "enforced": true }))
}

/// The error for `fault`, with `socket` naming the compositor's Wayland
/// socket.
pub(crate) fn capture_error(fault: &CaptureFault, socket: &Path) -> AdapterError {
    match fault {
        CaptureFault::Setup(wayland) => wayland.clone().into_error(socket),
        CaptureFault::FrameGone => display_cause_error(&DisplayCause::Missing {
            address: format!("{} closed the connection", socket.display()),
        }),
        CaptureFault::FrameSilent => {
            AdapterError::timeout("The compositor sent no frame before the deadline")
                .with_suggestion("Retry once the compositor draws again")
                .with_details(json!({ "kind": "capture_frame" }))
        }
        CaptureFault::Unsupported(interface) => AdapterError::new(
            ErrorCode::ActionNotSupported,
            format!("The compositor does not advertise {interface}, which capture needs"),
        )
        .with_suggestion("Update Hyprland to a release that offers it")
        .with_platform_detail(format!("missing Wayland global {interface}")),
        CaptureFault::Failed(FrameFailure::Stopped) => AdapterError::new(
            ErrorCode::WindowNotFound,
            "The window closed before its frame was captured",
        )
        .with_suggestion("Run 'list-windows' for the current window ids, then retry"),
        CaptureFault::Failed(failure) => failed("The compositor refused the frame")
            .with_details(json!({ "kind": "capture_refused", "reason": refusal_reason(*failure) })),
        CaptureFault::NoFormat => failed("The compositor offered no buffer layout this tool reads"),
        CaptureFault::Stale => failed("The compositor sent a frame from before the request"),
        CaptureFault::OutputMissing(name) => AdapterError::new(
            ErrorCode::InvalidArgs,
            format!("Display {name} is not one the compositor offers for capture now"),
        )
        .with_suggestion("Run 'list-displays' to see the displays that exist now")
        .with_details(json!({ "display": name })),
    }
}

fn refusal_reason(failure: FrameFailure) -> &'static str {
    match failure {
        FrameFailure::Unknown => "unknown",
        FrameFailure::BufferConstraints => "buffer_constraints",
        FrameFailure::Stopped => "stopped",
    }
}

fn failed(message: &str) -> AdapterError {
    AdapterError::new(ErrorCode::ActionFailed, message)
        .with_suggestion("Retry; nothing was captured")
}

#[cfg(test)]
#[path = "capture_error_tests.rs"]
mod tests;
