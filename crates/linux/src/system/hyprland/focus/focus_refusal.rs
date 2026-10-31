//! Refusals of Hyprland `focus-window` once the window is bound.

use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode, FocusSideEffect};
use serde_json::{Value, json};

use super::proof_read::ProofRead;
use crate::system::focus::focus_cause::Cause;
use crate::system::hyprland::clients::monitor_workspaces::MonitorWorkspaces;

/// A headless command found the window unfocused. It activates nothing, so
/// nothing was sent. `hidden_special` names the hidden special workspace
/// the window is on, which only a headed command reveals.
pub(crate) fn not_focused(hidden_special: Option<&str>) -> AdapterError {
    let (message, suggestion, details) = match hidden_special {
        Some(workspace) => (
            format!(
                "The window is on the hidden special workspace '{workspace}', and a headless focus-window does not reveal it"
            ),
            "Pass --headed to reveal the special workspace and activate the window".to_owned(),
            json!({"physical_delivery_started": false, "cause": "hidden_special_workspace", "workspace": workspace}),
        ),
        None => (
            "The window is not focused, and a headless focus-window does not activate windows"
                .to_owned(),
            "Pass --headed to activate the window".to_owned(),
            json!({"physical_delivery_started": false, "cause": "window_not_focused"}),
        ),
    };
    AdapterError::new(ErrorCode::PolicyDenied, message)
        .with_suggestion(suggestion)
        .with_details(details)
        .with_disposition(DeliverySemantics::not_delivered())
}

/// A special workspace is open over the monitor that shows the window's
/// workspace. Hyprland keeps it in front of the window it activates, on
/// 0.56.2 and 0.57.0 alike, so no activation is sent.
pub(crate) fn special_open(covering: &MonitorWorkspaces) -> AdapterError {
    let workspace = covering.special.as_deref().unwrap_or_default();
    let monitor = covering.monitor.as_str();
    AdapterError::new(
        ErrorCode::ActionFailed,
        format!(
            "The special workspace '{workspace}' is open over monitor {monitor}, and Hyprland keeps it in front of the window, so no activation was sent"
        ),
    )
    .with_suggestion(format!(
        "Close the special workspace '{workspace}', then retry"
    ))
    .with_details(json!({
        "physical_delivery_started": false,
        "cause": "special_workspace_open",
        "workspace": workspace,
        "monitor": monitor,
    }))
    .with_disposition(DeliverySemantics::not_delivered())
}

/// The focus witnesses did not all agree. After an activation was sent the
/// outcome is not safe to repeat blindly, and the side effects already
/// observed are reported.
pub(crate) fn proof_refusal(
    read: ProofRead,
    activation_sent: bool,
    side_effects: &[FocusSideEffect],
    read_error: Option<&AdapterError>,
) -> AdapterError {
    let cause = if read.compositor_active && read.toplevel_activated && !read.in_view {
        Cause::WindowHidden
    } else if read.compositor_active && read.toplevel_activated {
        Cause::FrameNotActive
    } else if activation_sent {
        Cause::ActivationRefused
    } else {
        Cause::ActivationNotGranted
    };
    let mut details = json!({
        "physical_delivery_started": false,
        "cause": cause.as_str(),
        "activation_sent": activation_sent,
        "compositor_active": read.compositor_active,
        "toplevel_activated": read.toplevel_activated,
        "in_view": read.in_view,
    });
    if let Some(fields) = details.as_object_mut() {
        if let Some(frame_active) = read.frame_active {
            fields.insert("frame_active".into(), Value::Bool(frame_active));
        }
        if !side_effects.is_empty() {
            fields.insert("side_effects".into(), json!(side_effects));
        }
        if let Some(error) = read_error {
            fields.insert(
                "last_read_error".into(),
                Value::String(error.message.clone()),
            );
        }
    }
    let disposition = if activation_sent {
        DeliverySemantics::delivered_unverified()
    } else {
        DeliverySemantics::not_delivered()
    };
    let message = if activation_sent {
        format!("{}; the activation was sent", cause_message(cause))
    } else {
        cause_message(cause).to_owned()
    };
    AdapterError::new(ErrorCode::ActionFailed, message)
        .with_suggestion(cause.suggestion())
        .with_details(details)
        .with_disposition(disposition)
}

fn cause_message(cause: Cause) -> &'static str {
    match cause {
        Cause::FrameNotActive => {
            "Hyprland made the window active but the application does not report it active"
        }
        Cause::ActivationRefused => "Hyprland left another window active",
        Cause::WindowHidden => {
            "Hyprland reports the window active, but no monitor shows its workspace"
        }
        _ => "Hyprland and its foreign toplevel state disagree about which window is active",
    }
}
