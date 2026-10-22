use std::time::Duration;

use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, DisplayInfo, ErrorCode, InteractionLease,
    InteractionPolicy, MouseButton, MouseEvent, MouseEventKind, Point, Rect,
};
use serde_json::json;

use super::backend::{Backend, backend};
use super::deliver::mouse_event;
use super::shell_aim::{aim, monitor_under};
use super::wayland::key_session::KeySession;
use super::wayland::remote_desktop::RemoteDesktop;
use crate::actions::physical_live_world::LiveWorld;
use crate::actions::physical_target::PhysicalWorld;
use crate::actions::physical_target::{ensure_budget, ensure_headed_click_policy};
use crate::system::shell::light::live_extents;
use crate::system::{SharedBus, list_displays};
use crate::tree::AtspiElement;

const SETTLE_PAUSE: Duration = Duration::from_millis(150);
const PRESS_PAUSE: Duration = Duration::from_millis(60);
const RECOVERY_MS: u64 = 1_000;
const BTN_LEFT: u32 = 272;
const BTN_RIGHT: u32 = 273;
const BTN_MIDDLE: u32 = 274;
pub(crate) trait PointerSession: KeySession {
    fn button(&mut self, button: u32, down: bool, deadline: Deadline) -> Result<(), AdapterError>;
    fn pause(&mut self, duration: Duration) {
        std::thread::sleep(duration);
    }
    fn attach_stream(&mut self, connector: &str, deadline: Deadline) -> Result<(), AdapterError>;
    fn motion_absolute(&mut self, x: f64, y: f64, deadline: Deadline) -> Result<(), AdapterError>;
}
impl PointerSession for RemoteDesktop {
    fn button(&mut self, button: u32, down: bool, deadline: Deadline) -> Result<(), AdapterError> {
        self.pointer_button(button, down, deadline)
    }

    fn attach_stream(&mut self, connector: &str, deadline: Deadline) -> Result<(), AdapterError> {
        RemoteDesktop::attach_stream(self, connector, deadline)
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

fn press<S: PointerSession>(
    session: &mut S,
    code: u32,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    session.button(code, true, deadline).map_err(|error| {
        if error.disposition == DeliverySemantics::not_delivered() {
            error
        } else {
            delivery_error(error, DeliverySemantics::uncertain())
        }
    })?;
    session.pause(PRESS_PAUSE);
    session
        .button(code, false, recovery_deadline(deadline))
        .map_err(|error| delivery_error(error, DeliverySemantics::uncertain()))
}

fn delivery_error(error: AdapterError, disposition: DeliverySemantics) -> AdapterError {
    let details = error.details.clone().unwrap_or_else(|| json!({}));
    let mut details = match details {
        serde_json::Value::Object(details) => details,
        other => serde_json::Map::from_iter([("cause".to_owned(), other)]),
    };
    details.insert("physical_delivery_started".to_owned(), json!(true));
    error
        .with_details(serde_json::Value::Object(details))
        .with_disposition(disposition)
        .with_suggestion("Inspect the target with 'snapshot' before retrying; a button press may already have been delivered")
}

fn execute_created_session<S: PointerSession>(
    session: &mut S,
    deadline: Deadline,
    execute: impl FnOnce(&mut S) -> Result<(), AdapterError>,
) -> Result<(), AdapterError> {
    match (execute(session), session.stop(recovery_deadline(deadline))) {
        (Ok(()), Err(error)) => Err(delivery_error(
            error,
            DeliverySemantics::delivered_unverified(),
        )),
        (result, _) => result,
    }
}

/// Release and Stop get their own budget after a click exhausts its deadline.
fn recovery_deadline(spent: Deadline) -> Deadline {
    Deadline::detached_after(RECOVERY_MS).unwrap_or(spent)
}

pub(crate) fn run_pointer<S: PointerSession>(
    session: &mut S,
    target: &Point,
    displays: &[DisplayInfo],
    button: &MouseButton,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    let (connector, local) = monitor_under(target, displays).ok_or_else(unreadable_extents)?;
    session.create(deadline)?;
    execute_created_session(session, deadline, |session| {
        session
            .attach_stream(&connector, deadline)
            .map_err(|mut error| {
                error.details.get_or_insert_with(|| json!({}))["physical_delivery_started"] =
                    false.into();
                error.with_disposition(DeliverySemantics::not_delivered())
            })?;
        session.start(deadline).map_err(|mut error| {
            error.details.get_or_insert_with(|| json!({}))["physical_delivery_started"] =
                false.into();
            error.with_disposition(DeliverySemantics::not_delivered())
        })?;
        session.motion_absolute(local.x, local.y, deadline)?;
        session.pause(SETTLE_PAUSE);
        press(session, button_code(button), deadline)
    })
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

fn deliver(
    point: &Point,
    displays: &[DisplayInfo],
    button: &MouseButton,
    lease: &InteractionLease,
) -> Result<(), AdapterError> {
    let deadline = lease.deadline();
    match backend()? {
        Backend::X11(_) => mouse_event(
            &MouseEvent {
                kind: MouseEventKind::Click { count: 1 },
                point: point.clone(),
                button: button.clone(),
                modifiers: Vec::new(),
            },
            lease,
        )
        .map(|_| ()),
        Backend::Wayland => {
            let mut session = RemoteDesktop::open(deadline)?;
            run_pointer(&mut session, point, displays, button, deadline)
        }
    }
}

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
    LiveWorld { bus, lease }.validate_pointer(node, &point, deadline)?;
    deliver(&point, &displays, &button, lease)?;
    Ok(point)
}

#[cfg(test)]
#[path = "shell_click_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "shell_click_live_tests.rs"]
mod live_tests;
