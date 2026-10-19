use std::time::Duration;

use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, DisplayInfo, ErrorCode, InteractionLease,
    InteractionPolicy, MouseButton, MouseEvent, MouseEventKind, Point, Rect,
};
use serde_json::json;

use super::backend::{Backend, backend};
use super::deliver::mouse_event;
use super::shell_aim::aim;
use super::wayland::key_session::KeySession;
use super::wayland::remote_desktop::RemoteDesktop;
use crate::actions::physical_target::{ensure_budget, ensure_headed_click_policy};
use crate::system::shell::light::live_extents;
use crate::system::{SharedBus, list_displays};
use crate::tree::AtspiElement;

const CLAMP_MOTIONS: usize = 4;
const CLAMP_DELTA: f64 = -4_000.0;
const STEP_PIXELS: f64 = 40.0;
const MOTION_PAUSE: Duration = Duration::from_millis(10);
const SETTLE_PAUSE: Duration = Duration::from_millis(150);
const PRESS_PAUSE: Duration = Duration::from_millis(60);
const RECOVERY_MS: u64 = 1_000;
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
    fn attach_stream(&mut self, _deadline: Deadline) -> bool {
        false
    }
    fn motion_absolute(
        &mut self,
        _x: f64,
        _y: f64,
        _deadline: Deadline,
    ) -> Result<(), AdapterError> {
        Err(AdapterError::internal("Absolute motion is not available"))
    }
}

impl PointerSession for RemoteDesktop {
    fn motion(&mut self, dx: f64, dy: f64, deadline: Deadline) -> Result<(), AdapterError> {
        self.pointer_motion_relative(dx, dy, deadline)
    }

    fn button(&mut self, button: u32, down: bool, deadline: Deadline) -> Result<(), AdapterError> {
        self.pointer_button(button, down, deadline)
    }

    fn attach_stream(&mut self, deadline: Deadline) -> bool {
        RemoteDesktop::attach_stream(self, deadline)
    }

    fn motion_absolute(&mut self, x: f64, y: f64, deadline: Deadline) -> Result<(), AdapterError> {
        self.pointer_motion_absolute(x, y, deadline)
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
    session.button(code, false, recovery_deadline(deadline))
}

/// A press that went down must come up, and a session that was created must
/// stop, even when the click spent its whole budget: both run on a short
/// deadline of their own, or the compositor keeps the button held.
fn recovery_deadline(spent: Deadline) -> Deadline {
    Deadline::detached_after(RECOVERY_MS).unwrap_or(spent)
}

/// Clicks at `local`, the target in the primary monitor's own coordinates,
/// with absolute motion on a linked screen-cast stream, which a modal that
/// holds a pointer grab (the open message list) honours. When the stream
/// cannot be linked or started, nothing was sent and the click is made with
/// relative motion to `target` instead.
pub(crate) fn run_pointer_absolute<S: PointerSession>(
    session: &mut S,
    target: &Point,
    local: &Point,
    button: &MouseButton,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    session.create(deadline)?;
    if session.attach_stream(deadline) && session.start(deadline).is_ok() {
        let delivered = session
            .motion_absolute(local.x, local.y, deadline)
            .and_then(|()| {
                session.pause(SETTLE_PAUSE);
                press(session, button_code(button), deadline)
            });
        let stopped = session.stop(recovery_deadline(deadline));
        return delivered.and(stopped);
    }
    let _ = session.stop(recovery_deadline(deadline));
    run_pointer(session, target, button, deadline)
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
    let stopped = session.stop(recovery_deadline(deadline));
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
    live_extents(bus, node, deadline)?
        .map(Rect::from)
        .ok_or_else(unreadable_extents)
}

/// The point in the primary monitor's own coordinates, when it lies inside
/// that monitor: the stream a click is aimed on records the primary monitor.
pub(crate) fn local_in_primary(point: &Point, displays: &[DisplayInfo]) -> Option<Point> {
    let primary = displays.iter().find(|display| display.is_primary)?;
    let bounds = &primary.bounds;
    let inside = point.x >= bounds.x
        && point.x < bounds.x + bounds.width
        && point.y >= bounds.y
        && point.y < bounds.y + bounds.height;
    inside.then_some(Point {
        x: point.x - bounds.x,
        y: point.y - bounds.y,
    })
}

fn deliver(
    point: &Point,
    displays: &[DisplayInfo],
    button: &MouseButton,
    deadline: Deadline,
) -> Result<(), AdapterError> {
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
            match local_in_primary(point, displays) {
                Some(local) => run_pointer_absolute(&mut session, point, &local, button, deadline),
                None => run_pointer(&mut session, point, button, deadline),
            }
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
    deliver(&point, &displays, &button, deadline)?;
    Ok(point)
}

#[cfg(test)]
#[path = "shell_click_tests.rs"]
mod tests;
