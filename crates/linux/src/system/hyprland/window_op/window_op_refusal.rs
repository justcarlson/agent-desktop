//! The errors of a Hyprland window operation. A refusal is decided before
//! anything is sent; a rejection is the compositor's answer to a request;
//! an unverified result is a request whose read-back never held.

use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode};
use serde_json::{Value, json};

use super::client_state::ClientState;
use super::window_step::WindowStep;

/// A request refused before anything was sent.
pub(crate) fn refused(cause: &str, message: &str, suggestion: &str) -> AdapterError {
    AdapterError::new(ErrorCode::ActionNotSupported, message)
        .with_suggestion(suggestion)
        .with_details(json!({ "cause": cause, "physical_delivery_started": false }))
        .with_disposition(DeliverySemantics::not_delivered())
}

/// The window has no client in the compositor's client list.
pub(crate) fn window_gone() -> AdapterError {
    AdapterError::new(
        ErrorCode::WindowNotFound,
        "The window is no longer in the compositor's client list",
    )
    .with_suggestion("Run 'list-windows' to find the window again")
    .with_details(json!({ "cause": "window_destroyed", "physical_delivery_started": false }))
    .with_disposition(DeliverySemantics::not_delivered())
}

/// The compositor answered `reply` to a request instead of `ok`, and so
/// applied nothing for it. `started` says whether an earlier request of the
/// same operation was applied.
pub(crate) fn rejected(reply: &str, started: bool) -> AdapterError {
    let disposition = if started {
        DeliverySemantics::delivered_unverified()
    } else {
        DeliverySemantics::not_delivered()
    };
    AdapterError::new(
        ErrorCode::ActionFailed,
        "The compositor rejected the window request",
    )
    .with_suggestion("Run 'list-windows' to see the window as it is now, then retry")
    .with_platform_detail(reply.trim())
    .with_details(json!({
        "cause": "compositor_rejected",
        "reply": reply.trim(),
        "physical_delivery_started": started,
    }))
    .with_disposition(disposition)
}

/// A request the compositor accepted whose read-back never held.
pub(crate) fn unverified(step: &WindowStep, observed: Option<&ClientState>) -> AdapterError {
    let (cause, message) = match observed {
        Some(_) => (
            "readback_mismatch",
            "The compositor left the window other than requested",
        ),
        None => (
            "readback_unavailable",
            "The window could not be read back after the request, so its effect is unknown",
        ),
    };
    let requested: Vec<Value> = step.goals.iter().map(|goal| goal.to_json()).collect();
    AdapterError::new(ErrorCode::ActionFailed, message)
        .with_suggestion(
            "Run 'list-windows' to see the window as it is now; Hyprland holds a window at its minimum size",
        )
        .with_details(json!({
            "cause": cause,
            "requested": requested,
            "observed": observed.map_or(Value::Null, ClientState::to_json),
        }))
        .with_disposition(DeliverySemantics::delivered_unverified())
}
