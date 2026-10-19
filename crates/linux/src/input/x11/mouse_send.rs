#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the module is laid out ahead of the code that calls it"
    )
)]

use agent_desktop_core::{AdapterError, Deadline, MouseEvent, MouseEventKind, Point};

use super::session;
use crate::input::pointer_plan::plan_click;
use crate::input::standalone::reject_standalone;
use crate::input::step::{Plan, Step};
use crate::input::wheel_plan::plan_wheel;

pub(crate) fn pointer_target(point: Point) -> Result<(i32, i32), AdapterError> {
    point.validate()?;
    Ok((point.x.round() as i32, point.y.round() as i32))
}

pub(crate) fn move_plan(point: Point) -> Result<Plan, AdapterError> {
    let (x, y) = pointer_target(point)?;
    Ok(Plan {
        steps: vec![Step::MoveTo { x, y }],
        release_origin: None,
    })
}

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
        MouseEventKind::Move => move_plan(event.point.clone()).map(|plan| vec![plan]),
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
