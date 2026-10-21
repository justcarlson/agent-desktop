use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};
use serde_json::json;

use super::AtspiElement;
use super::live_read::{NodeReader, read_live};
use super::state_bits::{MODAL, SHOWING, StateBits};

fn unknown() -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "Cannot establish whether a modal dialog blocks the target window",
    )
    .with_suggestion("Inspect the application's dialogs, act in the dialog or close it, then refresh the snapshot and retry")
    .with_details(json!({
        "check": "modal",
        "cause": "modal_state_unknown",
        "retryable": false,
        "physical_delivery_started": false,
        "semantic_delivery_started": false,
    }))
    .with_disposition(DeliverySemantics::not_delivered())
}

fn unreadable(error: AdapterError) -> AdapterError {
    unknown().with_platform_detail(format!("{}: {}", error.code.as_str(), error.message))
}

/// Semantic actions can bypass toolkit input grabs. A showing modal in the
/// application blocks other frames even when their controls remain enabled.
/// Without a proven owner relationship, other same-app frames are refused.
/// Shell surfaces are compositor controls, not application top-level windows.
pub(crate) fn ensure_unblocked(
    reader: &mut impl NodeReader,
    target: &AtspiElement,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    if reader.is_verified_shell(target, deadline) {
        return Ok(());
    }
    let owner = target.menu_owner();
    let frame = owner
        .as_ref()
        .or_else(|| target.frame())
        .ok_or_else(unknown)?;
    let windows = reader.top_levels(target, deadline).map_err(unreadable)?;
    if !windows.iter().any(|window| window == frame) {
        return Err(unknown());
    }
    for mut window in windows {
        if &window == frame {
            continue;
        }
        window.set_ancestry(super::ancestry::of_root(&window));
        let node = read_live(reader, &window.into_native_handle()).map_err(unreadable)?;
        let words = node.read.node.traits.states.present().ok_or_else(unknown)?;
        if words.is_empty() {
            return Err(unknown());
        }
        let bits = StateBits::new(words);
        if bits.has(SHOWING) && bits.has(MODAL) {
            let dialog = node
                .read
                .node
                .properties
                .name
                .non_blank()
                .map(String::as_str)
                .unwrap_or("unnamed dialog");
            return Err(AdapterError::new(
                ErrorCode::ActionFailed,
                format!("Target window is blocked by modal dialog '{dialog}'"),
            )
            .with_suggestion(format!(
                "Act in the dialog '{dialog}' or close it, then retry the original target"
            ))
            .with_details(json!({
                "check": "modal",
                "cause": "blocked_by_modal_dialog",
                "retryable": false,
                "dialog": dialog,
                "physical_delivery_started": false,
                "semantic_delivery_started": false,
            }))
            .with_disposition(DeliverySemantics::not_delivered()));
        }
    }
    Ok(())
}
