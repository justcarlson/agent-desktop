use std::time::Duration;

use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, ErrorCode, InteractionLease, InteractionPolicy,
    MouseButton, MouseEvent, MouseEventKind, Point, Rect,
};
use serde_json::json;

use super::backend::{Backend, backend};
use super::deliver::mouse_event;
use super::shell_aim::aim;
use super::wayland::key_session::KeySession;
use super::wayland::remote_desktop::RemoteDesktop;
use crate::actions::physical_target::{ensure_budget, ensure_headed_click_policy};
use crate::system::shell::light::extents_of;
use crate::system::{SharedBus, list_displays, within};
use crate::tree::AtspiElement;

const CLAMP_MOTIONS: usize = 4;
const CLAMP_DELTA: f64 = -4_000.0;
const STEP_PIXELS: f64 = 40.0;
const MOTION_PAUSE: Duration = Duration::from_millis(10);
const SETTLE_PAUSE: Duration = Duration::from_millis(150);
const PRESS_PAUSE: Duration = Duration::from_millis(60);
const BTN_LEFT: u32 = 272;
const BTN_RIGHT: u32 = 273;
const BTN_MIDDLE: u32 = 274;

/// A compositor pointer session: the keyboard session's lifecycle plus
/// relative motion and button edges.
pub(crate) trait PointerSession: KeySession {
    fn motion(&mut self, dx: f64, dy: f64, deadline: Deadline) -> Result<(), AdapterError>;
    fn button(&mut self, button: u32, down: bool, deadline: Deadline) -> Result<(), AdapterError>;
    fn pause(&mut self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

impl PointerSession for RemoteDesktop {
    fn motion(&mut self, dx: f64, dy: f64, deadline: Deadline) -> Result<(), AdapterError> {
        self.pointer_motion_relative(dx, dy, deadline)
    }

    fn button(&mut self, button: u32, down: bool, deadline: Deadline) -> Result<(), AdapterError> {
        self.pointer_button(button, down, deadline)
    }
}

fn button_code(button: &MouseButton) -> u32 {
    match button {
        MouseButton::Left => BTN_LEFT,
        MouseButton::Right => BTN_RIGHT,
        MouseButton::Middle => BTN_MIDDLE,
    }
}

fn travel<S: PointerSession>(
    session: &mut S,
    target: &Point,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    for _ in 0..CLAMP_MOTIONS {
        session.motion(CLAMP_DELTA, CLAMP_DELTA, deadline)?;
        session.pause(MOTION_PAUSE);
    }
    let (mut x, mut y) = (0.0_f64, 0.0_f64);
    let (goal_x, goal_y) = (target.x.round(), target.y.round());
    while (x, y) != (goal_x, goal_y) {
        let dx = (goal_x - x).clamp(-STEP_PIXELS, STEP_PIXELS);
        let dy = (goal_y - y).clamp(-STEP_PIXELS, STEP_PIXELS);
        session.motion(dx, dy, deadline)?;
        x += dx;
        y += dy;
        session.pause(MOTION_PAUSE);
    }
    Ok(())
}

fn press<S: PointerSession>(
    session: &mut S,
    code: u32,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    session.button(code, true, deadline)?;
    session.pause(PRESS_PAUSE);
    session.button(code, false, deadline)
}

/// Moves the compositor pointer to `target` and clicks. The session is
/// stopped on every path once it was created; a refused creation sends no
/// event.
pub(crate) fn run_pointer<S: PointerSession>(
    session: &mut S,
    target: &Point,
    button: &MouseButton,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    session.create(deadline)?;
    let delivered = session
        .start(deadline)
        .and_then(|()| travel(session, target, deadline))
        .and_then(|()| {
            session.pause(SETTLE_PAUSE);
            press(session, button_code(button), deadline)
        });
    let stopped = session.stop(deadline);
    delivered.and(stopped)
}

fn unreadable_extents() -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "The screen position of the shell control could not be read, so no event was sent",
    )
    .with_suggestion("Run 'snapshot' to refresh, then retry")
    .with_details(json!({
        "check": "geometry",
        "physical_delivery_started": false,
    }))
    .with_disposition(DeliverySemantics::not_delivered())
}

fn read_extents(
    bus: &SharedBus,
    node: &AtspiElement,
    deadline: Deadline,
) -> Result<Rect, AdapterError> {
    let connection = bus.get(deadline)?.inner().clone();
    let extents = within(deadline, async { Ok(extents_of(&connection, node).await) })?;
    extents.map(Rect::from).ok_or_else(unreadable_extents)
}

fn deliver(point: &Point, button: &MouseButton, deadline: Deadline) -> Result<(), AdapterError> {
    match backend()? {
        Backend::X11(_) => mouse_event(
            &MouseEvent {
                kind: MouseEventKind::Click { count: 1 },
                point: point.clone(),
                button: button.clone(),
                modifiers: Vec::new(),
            },
            deadline,
        ),
        Backend::Wayland => {
            let mut session = RemoteDesktop::open(deadline)?;
            run_pointer(&mut session, point, button, deadline)
        }
    }
}

/// A pointer click at the centre of a shell control, headed only. The point
/// is validated against the displays before any event is sent.
pub(crate) fn click(
    bus: &SharedBus,
    node: &AtspiElement,
    button: MouseButton,
    policy: InteractionPolicy,
    lease: &InteractionLease,
) -> Result<Point, AdapterError> {
    ensure_headed_click_policy(policy)?;
    let deadline = lease.deadline();
    ensure_budget(deadline)?;
    let extents = read_extents(bus, node, deadline)?;
    let displays = list_displays(deadline)?;
    let point = aim(extents, &displays)?;
    deliver(&point, &button, deadline)?;
    Ok(point)
}

#[cfg(test)]
#[path = "shell_click_tests.rs"]
mod tests;
