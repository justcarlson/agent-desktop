use agent_desktop_core::{AdapterError, DeliverySemantics};

use super::Request;
use super::refusal;
use super::world::World;
use crate::system::close_plan::{Step, next};
use crate::system::liveness::Liveness;
use crate::system::x11::control::{ControlError, ControlRead, Toplevel};

const DIALOG_LOOK_EVERY: u32 = 4;

fn unless_gone(world: &impl World, error: AdapterError) -> Result<(), AdapterError> {
    if world.liveness() == Liveness::Gone {
        Ok(())
    } else {
        Err(error)
    }
}

fn not_delivered(error: AdapterError) -> AdapterError {
    error.with_disposition(DeliverySemantics::not_delivered())
}

fn failure_of(world: &impl World, request: &Request<'_>, error: ControlError) -> AdapterError {
    match error {
        ControlError::TimedOut => not_delivered(world.timeout_error()),
        other => refusal::send_failed(request, other),
    }
}

fn fan_out(world: &impl World, request: &Request<'_>, windows: &[u32]) -> Result<(), AdapterError> {
    let mut sent = 0_usize;
    let mut first_failure = None;
    for window in windows {
        if world.remaining().is_zero() {
            if sent > 0 {
                break;
            }
            return Err(not_delivered(world.timeout_error()));
        }
        match world.request_close(*window) {
            Ok(()) => sent += 1,
            Err(ControlError::BadWindow) => {}
            Err(error) => {
                first_failure.get_or_insert_with(|| failure_of(world, request, error));
            }
        }
    }
    if sent > 0 {
        return Ok(());
    }
    unless_gone(
        world,
        first_failure.unwrap_or_else(|| refusal::windowless(request)),
    )
}

fn request_close(world: &impl World, request: &Request<'_>) -> Result<bool, AdapterError> {
    let windows = match world.toplevels() {
        ControlRead::Ready(windows) => windows,
        ControlRead::NotX11 if world.on_wayland() => {
            return unless_gone(world, refusal::wayland_session(request)).map(|()| false);
        }
        ControlRead::NotX11 | ControlRead::NoManager => {
            return unless_gone(world, refusal::no_route(request)).map(|()| false);
        }
        ControlRead::Unavailable if world.remaining().is_zero() => {
            return Err(not_delivered(world.timeout_error()));
        }
        ControlRead::Unavailable => {
            return unless_gone(world, refusal::unavailable(request)).map(|()| false);
        }
    };
    match world.liveness() {
        Liveness::Gone => return Ok(false),
        Liveness::Stopped => return Err(AdapterError::app_unresponsive(request.name)),
        Liveness::Alive => {}
    }
    if windows.is_empty() {
        let refused = if world.on_wayland() {
            refusal::wayland_session(request)
        } else {
            refusal::windowless(request)
        };
        return unless_gone(world, refused).map(|()| false);
    }
    if windows.iter().any(refusal::is_dialog) {
        return Err(refusal::dialog_pending(request, &windows, false));
    }
    let ids: Vec<u32> = windows.iter().map(|window| window.id).collect();
    fan_out(world, request, &ids).map(|()| true)
}

/// How the wait for an exit ended.
enum Waited {
    Gone,
    Dialog(Vec<Toplevel>),
    TimedOut,
}

/// The wait for the exit. A dialog that opens meanwhile, the save prompt a
/// window raises in answer to its close request, ends the wait at once: the
/// process stays until someone answers it.
fn wait_gone(world: &impl World) -> Waited {
    let mut ticks = 0_u32;
    loop {
        if world.liveness() == Liveness::Gone {
            return Waited::Gone;
        }
        if ticks % DIALOG_LOOK_EVERY == 0
            && let ControlRead::Ready(windows) = world.toplevels()
            && windows.iter().any(refusal::is_dialog)
        {
            return Waited::Dialog(windows);
        }
        if world.remaining().is_zero() {
            return Waited::TimedOut;
        }
        world.pause();
        ticks += 1;
    }
}

/// The close that never signals: close requests to every top-level of the
/// process, then a wait for it to exit. A process that does not exit by the
/// deadline is a `TIMEOUT` with the request delivered and its effect
/// unverified.
pub(super) fn close(world: &impl World, request: &Request<'_>) -> Result<(), AdapterError> {
    let liveness = world.liveness();
    match next(false, liveness, None, std::time::Duration::ZERO) {
        Step::Done => return Ok(()),
        Step::Unresponsive => return Err(AdapterError::app_unresponsive(request.name)),
        Step::Request => {}
        Step::Term | Step::Kill | Step::Wait => {
            return Err(AdapterError::internal("a graceful close planned a signal"));
        }
    }
    if !request_close(world, request)? && world.liveness() == Liveness::Gone {
        return Ok(());
    }
    match wait_gone(world) {
        Waited::Gone => return Ok(()),
        Waited::Dialog(windows) => {
            return Err(refusal::dialog_pending(request, &windows, true));
        }
        Waited::TimedOut => {}
    }
    let remaining = world.toplevels_late().ready();
    Err(refusal::waited_out(
        world.timeout_error(),
        request,
        remaining.as_deref(),
    ))
}

#[cfg(test)]
#[path = "graceful_tests.rs"]
mod tests;
