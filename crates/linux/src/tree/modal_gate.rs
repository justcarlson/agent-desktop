use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};
use serde_json::json;

use super::AtspiElement;
use super::live_read::NodeReader;
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
    if error.code == ErrorCode::Internal {
        return error;
    }
    unknown().with_platform_detail(format!("{}: {}", error.code.as_str(), error.message))
}

/// Semantic actions can bypass toolkit input grabs. A showing modal in the
/// application blocks targets outside its subtree even when their controls
/// remain enabled. Unknown modal membership refuses delivery.
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
    let windows = reader.top_levels(frame, deadline).map_err(unreadable)?;
    let mut found_frame = false;
    for mut window in windows {
        window.set_ancestry(super::ancestry::of_root(&window));
        let walk = reader.native_tree(&window, deadline).map_err(unreadable)?;
        if !walk.complete || super::nested_dialog::is_cut_short(&walk.root) {
            return Err(unknown());
        }
        found_frame |= contains(&walk.root, frame);
        inspect(&walk.root, owner.as_ref().unwrap_or(target))?;
    }
    if !found_frame {
        return Err(unknown());
    }
    Ok(())
}

fn contains(node: &super::WalkedNode<super::ObservedRead>, target: &AtspiElement) -> bool {
    &node.payload.node.element == target
        || node.children.iter().any(|child| contains(child, target))
}

fn inspect(
    node: &super::WalkedNode<super::ObservedRead>,
    target: &AtspiElement,
) -> Result<(), AdapterError> {
    let raw = &node.payload.node;
    let role = raw.traits.role.present().copied().ok_or_else(unknown)?;
    if super::role_numbers::is_web_document(Some(role)) {
        return Ok(());
    }
    let words = raw
        .traits
        .states
        .present()
        .filter(|words| !words.is_empty())
        .ok_or_else(unknown)?;
    let bits = StateBits::new(words);
    if bits.has(SHOWING) && bits.has(MODAL) && !contains(node, target) {
        let dialog = raw
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
    for child in &node.children {
        inspect(child, target)?;
    }
    Ok(())
}
