use agent_desktop_core::{
    Action, ActionRequest, ActionResult, AdapterError, DeliverySemantics, ErrorCode,
    InteractionLease, NativeHandle,
};
use serde_json::json;

use super::click::{click, right_click};
use super::context::Target;
use super::disclosure::set_expanded;
use super::focus::focus;
use super::physical_delivery::physical_delivery;
use super::process_check::verify_process;
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

fn reaches_physical_without_verb(action: &Action) -> bool {
    matches!(
        action,
        Action::Click | Action::RightClick | Action::SetFocus | Action::TypeText(_)
    )
}

fn has_no_verb(error: &AdapterError) -> bool {
    error.code == ErrorCode::ActionNotSupported
        && error
            .details
            .as_ref()
            .and_then(|details| details.get("semantic_verb"))
            .is_some_and(|verb| verb == "absent")
}

fn name_stopped_target(
    mut error: AdapterError,
    pid: u32,
    is_stopped: impl Fn(u32) -> bool,
) -> AdapterError {
    let uncertain = error.disposition == DeliverySemantics::uncertain();
    if error.code == ErrorCode::Timeout && uncertain && is_stopped(pid) {
        let mut details = error.details.take().unwrap_or_else(|| json!({}));
        if let Some(map) = details.as_object_mut() {
            map.insert("target_stopped".to_owned(), json!(true));
        }
        error.details = Some(details);
    }
    error
}

fn falls_back_to_semantic(error: &AdapterError) -> bool {
    let unaimable = error
        .details
        .as_ref()
        .and_then(|details| details.get("check"))
        .is_some_and(|check| check == "geometry");
    error.disposition == DeliverySemantics::not_delivered()
        && (error.code == ErrorCode::ActionNotSupported || unaimable)
}

pub(crate) fn physical_first(
    physical: impl FnOnce() -> Result<ActionResult, AdapterError>,
    semantic: impl FnOnce() -> Result<ActionResult, AdapterError>,
) -> Result<ActionResult, AdapterError> {
    match physical() {
        Err(error) if falls_back_to_semantic(&error) => semantic(),
        other => other,
    }
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
    verify_process(bus, element, request.expected_process(), deadline)?;
    let target = Target {
        bus,
        handle,
        element,
        deadline,
    };
    let semantic = match &request.action {
        Action::Click if request.policy.is_headed() => physical_first(
            || physical_delivery(bus, &request.action, Some(element), &request, lease),
            || click(&target),
        ),
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
        Action::TypeText(text) if request.policy.is_headed() => physical_first(
            || physical_delivery(bus, &request.action, Some(element), &request, lease),
            || type_text(&target, text),
        ),
        Action::TypeText(text) => type_text(&target, text),
        Action::ScrollTo => scroll_to(bus, handle, lease),
        Action::DoubleClick | Action::TripleClick | Action::Scroll(_, _) | Action::PressKey(_) => {
            physical_delivery(bus, &request.action, Some(element), &request, lease)
        }
        Action::Hover | Action::Drag(_) | Action::KeyDown(_) | Action::KeyUp(_) => {
            Err(handled_by_core(&request.action))
        }
    };
    match semantic {
        Err(error) if reaches_physical_without_verb(&request.action) && has_no_verb(&error) => {
            physical_delivery(bus, &request.action, Some(element), &request, lease)
        }
        other => other.map_err(|error| {
            name_stopped_target(error, element.pid(), crate::system::proc_stat::is_stopped)
        }),
    }
}

#[cfg(test)]
#[path = "dispatch_tests.rs"]
mod tests;
