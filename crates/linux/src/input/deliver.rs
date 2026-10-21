use agent_desktop_core::{
    AdapterError, Deadline, DragParams, InteractionLease, MouseEvent, MouseEventKind,
    OperationReport,
};

use super::backend::{Backend, backend};
use super::delivery::{Delivery, Gate};
use super::drag_plan::plan_drag;
use super::keysym_step::KeysymStep;
use super::plan::Plan;
use super::pointer_report::with_event_count;
use super::wayland::keysym_send;
use super::x11::{mouse_send, session};

pub(crate) fn send_plans(
    plans: Vec<Plan>,
    gate: Gate<'_>,
    lease: &InteractionLease,
) -> Result<Delivery, AdapterError> {
    match backend()? {
        Backend::X11(display) => session::run(&display, plans, gate, lease),
        Backend::Wayland => Err(keysym_send::pointer_refusal("pointer_and_chord_input")),
    }
}

pub(crate) fn send_keysyms(
    chunks: &[Vec<KeysymStep>],
    gate: Gate<'_>,
    deadline: Deadline,
) -> Result<Delivery, AdapterError> {
    match backend()? {
        Backend::X11(_) => Err(AdapterError::not_supported("x11_keysym_input")),
        Backend::Wayland => keysym_send::send_chunks(chunks, gate, deadline),
    }
}

pub(crate) fn mouse_event(
    event: &MouseEvent,
    lease: &InteractionLease,
) -> Result<OperationReport, AdapterError> {
    match &event.kind {
        MouseEventKind::Down => {
            return Err(super::standalone::reject_standalone("mouse_event down"));
        }
        MouseEventKind::Up => return Err(super::standalone::reject_standalone("mouse_event up")),
        _ => {}
    }
    match backend()? {
        Backend::X11(display) => {
            let plans = mouse_send::plans_for(&display, event, lease)?;
            let delivery = session::run(&display, plans, &mut || Ok(()), lease)?;
            Ok(with_event_count(
                OperationReport::pointer_dispatched(event, false),
                &delivery,
            ))
        }
        Backend::Wayland => Err(keysym_send::pointer_refusal("mouse_event")),
    }
}

pub(crate) fn drag(
    params: &DragParams,
    lease: &InteractionLease,
) -> Result<OperationReport, AdapterError> {
    let deadline = lease.deadline();
    match backend()? {
        Backend::X11(display) => {
            let plan = plan_drag(params, deadline)?;
            let delivery = session::run(&display, vec![plan], &mut || Ok(()), lease)?;
            Ok(with_event_count(
                OperationReport::drag_dispatched(params),
                &delivery,
            ))
        }
        Backend::Wayland => Err(keysym_send::pointer_refusal("drag")),
    }
}
