use super::click::{POLL_STEP, POPUP_SETTLE, opens_popup, poll_until};
use super::context::Target;
use crate::system::menu::menu_surface::roots_below;
use crate::tree::AtspiElement;
use crate::tree::action_read::{NodeView, read_node_view};
use crate::tree::role_numbers::{PUSH_BUTTON, PUSH_BUTTON_MENU, TOGGLE_BUTTON};
use agent_desktop_core::{ActionResult, AdapterError, DeliverySemantics, ErrorCode};

fn opener(target: &Target<'_>, view: &NodeView) -> Result<Option<AtspiElement>, AdapterError> {
    if matches!(view.role(), Some(PUSH_BUTTON | PUSH_BUTTON_MENU)) && opens_popup(view) {
        return Ok(Some(target.element.clone()));
    }
    if view.role() != Some(TOGGLE_BUTTON) {
        return Ok(None);
    }
    let Some(parent) = target.element.ancestry().button_parent() else {
        return Ok(None);
    };
    let handle = parent.clone().into_native_handle();
    let view = read_node_view(target.bus, &handle, target.deadline)
        .map_err(|error| error.with_disposition(DeliverySemantics::not_delivered()))?;
    if view.state(crate::tree::state_bits::HAS_POPUP).is_none() {
        return Err(AdapterError::new(
            ErrorCode::ActionFailed,
            "The parent button's popup capability could not be read",
        )
        .with_disposition(DeliverySemantics::not_delivered()));
    }
    Ok(opens_popup(&view).then_some(parent))
}

fn matches(wanted: Option<bool>, before: &[AtspiElement], after: &[AtspiElement]) -> bool {
    match wanted {
        Some(open) => after.is_empty() != open,
        None => before != after,
    }
}

fn confirm(
    result: ActionResult,
    wanted: Option<bool>,
    before: &[AtspiElement],
    after: Option<&[AtspiElement]>,
) -> Result<ActionResult, AdapterError> {
    if after.is_some_and(|after| matches(wanted, before, after)) {
        return Ok(result);
    }
    let disposition = if result.disposition() == DeliverySemantics::not_delivered() {
        DeliverySemantics::not_delivered()
    } else {
        DeliverySemantics::delivered_unverified()
    };
    let mut steps = result.steps;
    for step in &mut steps {
        step.verified = Some(false);
    }
    Err(AdapterError::new(
        ErrorCode::ActionFailed,
        "The requested popup was not observed after the opener action",
    )
    .with_suggestion(
        "Inspect a fresh snapshot before taking another action; no automatic retry is safe.",
    )
    .with_details(serde_json::json!({
        "kind": "popup_not_observed",
        "action": result.action,
        "steps": steps,
        "popup_present": after.map(|roots| !roots.is_empty()),
    }))
    .with_disposition(disposition))
}

pub(super) fn run(
    target: &Target<'_>,
    view: &NodeView,
    wanted: Option<bool>,
    invoke: impl FnOnce() -> Result<ActionResult, AdapterError>,
) -> Result<ActionResult, AdapterError> {
    let Some(opener) = opener(target, view)? else {
        return invoke();
    };
    attempt(
        wanted,
        || roots_below(target.bus, &opener, target.deadline),
        invoke,
        |before| {
            poll_until(
                target.deadline,
                POPUP_SETTLE,
                POLL_STEP,
                || roots_below(target.bus, &opener, target.deadline),
                |after| {
                    after
                        .as_deref()
                        .is_some_and(|after| matches(wanted, before, after))
                },
            )
        },
    )
}

fn attempt(
    wanted: Option<bool>,
    read_before: impl FnOnce() -> Option<Vec<AtspiElement>>,
    invoke: impl FnOnce() -> Result<ActionResult, AdapterError>,
    read_after: impl FnOnce(&[AtspiElement]) -> Option<Vec<AtspiElement>>,
) -> Result<ActionResult, AdapterError> {
    let before = read_before().ok_or_else(|| {
        AdapterError::new(
            ErrorCode::ActionFailed,
            "The popup subtree could not be read before delivery",
        )
        .with_disposition(DeliverySemantics::not_delivered())
    })?;
    let result = invoke()?;
    let after = read_after(&before);
    confirm(result, wanted, &before, after.as_deref())
}

#[cfg(test)]
#[path = "popup_verify_tests.rs"]
mod tests;
