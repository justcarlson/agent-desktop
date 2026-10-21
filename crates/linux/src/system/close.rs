use std::time::Instant;

use agent_desktop_core::{AdapterError, AppInfo, InteractionLease, OperationReport};

use super::SharedBus;
use crate::system::process_lookup::Lookup;
use world::{Live, World};

#[cfg(test)]
mod fake;
mod force;
mod graceful;
mod refusal;
mod world;

/// What the caller asked to close: the listed application and whether to end
/// it without asking.
struct Request<'a> {
    name: &'a str,
    pid: u32,
    instance: Option<&'a str>,
    force: bool,
}

fn refuse_if_protected(world: &impl World, request: &Request<'_>) -> Result<(), AdapterError> {
    if world.is_protected(request.name) {
        return Err(refusal::protected(request.name));
    }
    match world.facts() {
        Lookup::Found(facts) => {
            let exe = facts.exe_basename.as_deref();
            [Some(facts.comm.as_str()), exe]
                .into_iter()
                .flatten()
                .find(|name| world.is_protected(name))
                .map_or(Ok(()), |name| Err(refusal::protected(name)))
        }
        Lookup::Gone => Ok(()),
        Lookup::Unreadable => Err(refusal::unreadable(request)),
    }
}

/// The close flow over a world. The protected check comes first, on the
/// listed name and on what `/proc` says the process is, then the token
/// requirement, then the process itself. A process that is gone, or that a
/// changed token shows is not the listed one, is `Ok` and untouched.
/// The result is true only if a close request or signal was delivered.
fn run(world: &impl World, request: &Request<'_>) -> Result<bool, AdapterError> {
    refuse_if_protected(world, request)?;
    if request.instance.is_none() {
        return Err(refusal::missing_token(request));
    }
    if request.force {
        force::close(world, request)
    } else {
        graceful::close(world, request)
    }
}

fn run_report(
    world: &impl World,
    app: &AppInfo,
    force: bool,
) -> Result<OperationReport, AdapterError> {
    let request = Request {
        name: &app.name,
        pid: app.pid.get(),
        instance: app.process_instance.as_deref(),
        force,
    };
    OperationReport::from_close_dispatch(app, run(world, &request))
}

pub(crate) fn close_app(
    _bus: &SharedBus,
    app: &AppInfo,
    force: bool,
    lease: &InteractionLease,
) -> Result<OperationReport, AdapterError> {
    let pid = app.pid.get();
    let live = Live {
        pid,
        instance: app.process_instance.as_deref().unwrap_or_default(),
        deadline: lease.deadline(),
        started: Instant::now(),
    };
    run_report(&live, app, force)
}

#[cfg(test)]
#[path = "close_tests.rs"]
mod tests;
