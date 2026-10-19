use agent_desktop_core::{AdapterError, Deadline, MouseEvent, MouseEventKind};

use super::session;
use crate::input::plan::Plan;
use crate::input::pointer_plan::{plan_click, plan_move};
use crate::input::standalone::reject_standalone;
use crate::input::wheel_plan::plan_wheel;

pub(crate) fn plans_for(
    display: &str,
    event: &MouseEvent,
    deadline: Deadline,
) -> Result<Vec<Plan>, AdapterError> {
    let map = if event.modifiers.is_empty() {
        Default::default()
    } else {
        session::live_keymap(display, deadline)?
    };
    match &event.kind {
        MouseEventKind::Move => plan_move(event.point.clone()).map(|plan| vec![plan]),
        MouseEventKind::Click { count } => plan_click(
            &map,
            event.point.clone(),
            &event.button,
            *count,
            &event.modifiers,
        )
        .map(|plan| vec![plan]),
        MouseEventKind::Wheel { delta_x, delta_y } => plan_wheel(
            &map,
            event.point.clone(),
            *delta_x,
            *delta_y,
            &event.modifiers,
        ),
        MouseEventKind::Down => Err(reject_standalone("mouse_event down")),
        MouseEventKind::Up => Err(reject_standalone("mouse_event up")),
    }
}

pub(crate) fn mouse_event(
    display: &str,
    event: &MouseEvent,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    let plans = plans_for(display, event, deadline)?;
    session::run(display, plans, &mut || Ok(()), deadline).map(|_| ())
}

#[cfg(test)]
#[path = "mouse_send_tests.rs"]
mod tests;
