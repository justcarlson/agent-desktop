//! Pointer input on the bound Hyprland instance: cursors hidden, the virtual
//! pointer opened on the bound socket, the active window held steady before
//! every press and wheel step, and the hide still held before every plan,
//! press and wheel step.

use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, DragParams, MouseEvent, OperationReport,
};
use serde_json::json;

use super::cursor_guard::{nothing_sent, with_cursors_hidden};
use super::focus_gate::FocusGate;
use super::live_pointer_sink::LivePointerSink;
use super::pointer_cancel::with_release_on_signal;
use super::pointer_run::PointerRun;
use super::run_plans::run_plans;
use crate::input::delivery::{Delivery, Gate};
use crate::input::mouse::drag_plan::plan_drag;
use crate::input::mouse::pointer_plan::{plan_click, plan_move};
use crate::input::mouse::wheel_plan::plan_wheel;
use crate::input::plan::Plan;
use crate::input::x11::progress::Progress;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::clients::client_parse::parse_active;
use crate::system::hyprland::cursor_context::CursorContext;
use crate::system::hyprland::cursor_renderer::hold_registry::ensure_held;
use crate::system::hyprland::ipc::bound_request::bound_request;

const ACTIVE_COMMAND: &str = "j/activewindow";

/// Sends `plans` through the bound instance's virtual pointer. `plan_gate`
/// runs before each plan, after the cursors are hidden. A signal stops the
/// run at its next event and releases its held buttons.
pub(crate) fn send(
    binding: &HyprlandBinding,
    plans: &[Plan],
    plan_gate: Gate<'_>,
    deadline: Deadline,
) -> Result<Delivery, AdapterError> {
    let slot = crate::input::x11::worker_slot::shared();
    with_release_on_signal(&slot, deadline, |progress| {
        send_hidden(binding, plans, plan_gate, progress, deadline)
    })
}

fn send_hidden(
    binding: &HyprlandBinding,
    plans: &[Plan],
    plan_gate: Gate<'_>,
    progress: &Progress,
    deadline: Deadline,
) -> Result<Delivery, AdapterError> {
    let mut hider = crate::system::hyprland::cursor_hider(&CursorContext { binding });
    with_cursors_hidden(hider.as_mut(), deadline, || {
        let displays = crate::system::list_displays(deadline).map_err(nothing_sent)?;
        let mut sink = LivePointerSink::open(binding, deadline).map_err(nothing_sent)?;
        let mut focus = FocusGate::new(|| read_active(binding, deadline), None);
        let mut focus_check = || focus.check();
        let delivery = run_plans(
            &mut sink,
            PointerRun {
                displays: &displays,
                plan_gate: &mut while_held(plan_gate),
                step_gate: &mut while_held(&mut focus_check),
                deadline,
                progress,
            },
            plans,
        )?;
        sink.finish()?;
        Ok(delivery)
    })
}

/// `gate` behind the check that this thread's cursor hide was not lost, so a
/// failed renewal stops the run before its next gated event.
pub(crate) fn while_held<'a>(gate: Gate<'a>) -> impl FnMut() -> Result<(), AdapterError> + 'a {
    move || {
        ensure_held()?;
        gate()
    }
}

/// A coordinate-only `mouse-move`, `mouse-click` or `mouse-wheel`. No
/// receiver is proven, so the report says `receiver_proof: false`.
pub(crate) fn mouse_event(
    binding: &HyprlandBinding,
    event: &MouseEvent,
    deadline: Deadline,
) -> Result<OperationReport, AdapterError> {
    let plans = event_plans(event)?;
    let delivery = send(binding, &plans, &mut || Ok(()), deadline)?;
    Ok(coordinate_report(
        OperationReport::pointer_dispatched(event, false),
        &delivery,
    ))
}

/// A coordinate-only `drag`, reported with `receiver_proof: false`.
pub(crate) fn drag(
    binding: &HyprlandBinding,
    params: &DragParams,
    deadline: Deadline,
) -> Result<OperationReport, AdapterError> {
    let plan = plan_drag(params, deadline)?;
    let delivery = send(binding, &[plan], &mut || Ok(()), deadline)?;
    Ok(coordinate_report(
        OperationReport::drag_dispatched(params),
        &delivery,
    ))
}

/// The plans for one coordinate event. Modifier keys need the keyboard
/// transport, so a request that names one is refused before any event.
pub(crate) fn event_plans(event: &MouseEvent) -> Result<Vec<Plan>, AdapterError> {
    use agent_desktop_core::MouseEventKind;
    if !event.modifiers.is_empty() {
        return Err(modifiers_refused());
    }
    let map = crate::input::keyboard::keymap::Keymap::default();
    match &event.kind {
        MouseEventKind::Move => Ok(vec![plan_move(event.point.clone())?]),
        MouseEventKind::Click { count } => {
            agent_desktop_core::validate_mouse_click_count(*count)?;
            Ok(vec![plan_click(
                &map,
                event.point.clone(),
                &event.button,
                *count,
                &[],
            )?])
        }
        MouseEventKind::Wheel { delta_x, delta_y } => {
            plan_wheel(&map, event.point.clone(), *delta_x, *delta_y, &[])
        }
        MouseEventKind::Down => Err(crate::input::standalone::reject_standalone(
            "mouse_event down",
        )),
        MouseEventKind::Up => Err(crate::input::standalone::reject_standalone(
            "mouse_event up",
        )),
    }
}

/// Adds the event count and `receiver_proof: false` to a coordinate report.
pub(crate) fn coordinate_report(report: OperationReport, delivery: &Delivery) -> OperationReport {
    OperationReport {
        result: report.result.with_details(json!({
            "delivered_events": delivery.delivered_events,
            "receiver_proof": false,
        })),
        target: report.target,
    }
}

fn modifiers_refused() -> AdapterError {
    AdapterError::new(
        agent_desktop_core::ErrorCode::ActionNotSupported,
        "Hyprland pointer input does not hold modifier keys",
    )
    .with_details(json!({
        "kind": "hyprland_pointer",
        "cause": "modifiers_unsupported",
        "physical_delivery_started": false,
    }))
    .with_suggestion("Send the pointer event without modifiers")
    .with_disposition(DeliverySemantics::not_delivered())
}

fn read_active(binding: &HyprlandBinding, deadline: Deadline) -> Result<Option<u64>, AdapterError> {
    let body = bound_request(binding, ACTIVE_COMMAND, deadline)?;
    parse_active(&body).ok_or_else(|| {
        AdapterError::internal("The Hyprland active window could not be read")
            .with_suggestion("Retry once the compositor answers")
    })
}

#[cfg(test)]
#[path = "hypr_pointer_tests.rs"]
mod tests;
