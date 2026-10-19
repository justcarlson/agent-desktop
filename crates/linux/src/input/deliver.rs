use agent_desktop_core::{AdapterError, Deadline, DragParams, MouseEvent, MouseEventKind};

use super::backend::{Backend, backend};
use super::delivery::Delivery;
use super::keysym_step::KeysymStep;
use super::plan::Plan;
use super::standalone::reject_standalone;
use super::wayland::keysym_send;
use super::x11::{drag_send, mouse_send, session};

pub(crate) fn send_plans(
    plans: Vec<Plan>,
    gate: &mut dyn FnMut() -> Result<(), AdapterError>,
    deadline: Deadline,
) -> Result<Delivery, AdapterError> {
    match backend()? {
        Backend::X11(display) => session::run(&display, plans, gate, deadline),
        Backend::Wayland => Err(keysym_send::pointer_refusal("pointer_and_chord_input")),
    }
}

pub(crate) fn send_keysyms(
    keys: &[KeysymStep],
    gate: &mut dyn FnMut() -> Result<(), AdapterError>,
    deadline: Deadline,
) -> Result<Delivery, AdapterError> {
    match backend()? {
        Backend::X11(_) => Err(AdapterError::not_supported("x11_keysym_input")),
        Backend::Wayland => keysym_send::send_keys(keys, gate, deadline),
    }
}

pub(crate) fn mouse_event(event: &MouseEvent, deadline: Deadline) -> Result<(), AdapterError> {
    match &event.kind {
        MouseEventKind::Down => return Err(reject_standalone("mouse_event down")),
        MouseEventKind::Up => return Err(reject_standalone("mouse_event up")),
        _ => {}
    }
    match backend()? {
        Backend::X11(display) => mouse_send::mouse_event(&display, event, deadline),
        Backend::Wayland => Err(keysym_send::pointer_refusal("mouse_event")),
    }
}

pub(crate) fn drag(params: &DragParams, deadline: Deadline) -> Result<(), AdapterError> {
    match backend()? {
        Backend::X11(display) => drag_send::drag(&display, params, deadline),
        Backend::Wayland => Err(keysym_send::pointer_refusal("drag")),
    }
}
