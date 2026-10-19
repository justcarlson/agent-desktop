use agent_desktop_core::{
    Action, ActionRequest, ActionResult, AdapterError, DeliverySemantics, ErrorCode,
    InteractionLease, NativeHandle,
};

use super::click::{click, right_click};
use super::context::Target;
use super::disclosure::set_expanded;
use super::focus::focus;
use super::physical_delivery::physical_delivery;
use super::scroll_wrap::scroll_to;
use super::select::select;
use super::toggle_state::toggle;
use super::value_write::{clear, set_value, type_text};
use crate::system::{SharedBus, ensure_budget};
use crate::tree::{AtspiElement, atspi_element};

fn handled_by_core(action: &Action) -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionNotSupported,
        format!("{} requires adapter-level handling", action.name()),
    )
    .with_disposition(DeliverySemantics::not_delivered())
}

fn target_element<'a>(
    handle: &'a NativeHandle,
    action: &Action,
) -> Result<Option<&'a AtspiElement>, AdapterError> {
    if matches!(action, Action::PressKey(_)) && handle.is_null() {
        return Ok(None);
    }
    atspi_element(handle).map(Some)
}

pub(crate) fn execute_action(
    bus: &SharedBus,
    handle: &NativeHandle,
    request: ActionRequest,
    lease: &InteractionLease,
) -> Result<ActionResult, AdapterError> {
    let deadline = lease.deadline();
    let element = target_element(handle, &request.action)?;
    ensure_budget(deadline)?;
    let Some(element) = element else {
        return physical_delivery(bus, &request.action, None, &request, lease);
    };
    let target = Target {
        bus,
        handle,
        element,
        deadline,
    };
    match &request.action {
        Action::Click => click(&target),
        Action::RightClick => right_click(&target),
        Action::Toggle => toggle(&target, None),
        Action::Check => toggle(&target, Some(true)),
        Action::Uncheck => toggle(&target, Some(false)),
        Action::Expand => set_expanded(&target, true),
        Action::Collapse => set_expanded(&target, false),
        Action::Select(display_value) => select(&target, display_value),
        Action::SetFocus => focus(&target, request.policy),
        Action::SetValue(text) => set_value(&target, text),
        Action::Clear => clear(&target),
        Action::TypeText(text) => type_text(&target, text),
        Action::ScrollTo => scroll_to(bus, handle, lease),
        Action::DoubleClick | Action::TripleClick | Action::Scroll(_, _) | Action::PressKey(_) => {
            physical_delivery(bus, &request.action, Some(element), &request, lease)
        }
        Action::Hover | Action::Drag(_) | Action::KeyDown(_) | Action::KeyUp(_) => {
            Err(handled_by_core(&request.action))
        }
    }
}

#[cfg(test)]
#[path = "dispatch_tests.rs"]
mod tests;
