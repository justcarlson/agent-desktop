use agent_desktop_core::{
    ActionResult, AdapterError, Deadline, DeliverySemantics, ErrorCode, InteractionLease,
    InteractionPolicy, KeyCombo, ProcessIdentity,
};
use serde_json::json;

use super::SharedBus;
use super::listed_app_name;
use super::proc_stat;
use super::process_facts::{self, Lookup};
use super::window_ops::focused_window;
use super::x11::stacking_source::StackingSource;
use super::x11::{DisplayReader, X11State};
use crate::actions::physical_keyboard::press_for_process;
use crate::actions::physical_target::{LiveWorld, PhysicalWorld};
use crate::input::backend::Backend;

/// What the key gate reads about the desktop, so the decision can be driven
/// by a fixed desktop.
trait KeyWorld {
    fn lookup(&self, pid: u32) -> Lookup;
    fn stopped(&self, pid: u32) -> bool;
    fn focused_pid(&self, deadline: Deadline) -> Result<Option<u32>, AdapterError>;
    fn active_client_pid(&self, deadline: Deadline) -> Option<u32>;
}

struct LiveKeys<'a> {
    bus: &'a SharedBus,
}

impl KeyWorld for LiveKeys<'_> {
    fn lookup(&self, pid: u32) -> Lookup {
        process_facts::lookup(pid)
    }

    fn stopped(&self, pid: u32) -> bool {
        proc_stat::is_stopped(pid)
    }

    fn focused_pid(&self, deadline: Deadline) -> Result<Option<u32>, AdapterError> {
        Ok(focused_window(self.bus, deadline)?.map(|window| window.pid.get()))
    }

    fn active_client_pid(&self, deadline: Deadline) -> Option<u32> {
        let X11State::Ready { active, clients } = DisplayReader.clients_within(deadline) else {
            return None;
        };
        let active = active?;
        clients
            .iter()
            .find(|client| client.id == active)
            .and_then(|client| client.pid)
    }
}

fn process_gone(process: &ProcessIdentity) -> AdapterError {
    AdapterError::new(
        ErrorCode::AppUnresponsive,
        "Target process instance is no longer running",
    )
    .with_details(json!({
        "pid": process.pid,
        "process_instance": process.instance,
        "physical_delivery_started": false,
    }))
    .with_disposition(DeliverySemantics::not_delivered())
}

fn require_process(world: &impl KeyWorld, process: &ProcessIdentity) -> Result<(), AdapterError> {
    match world.lookup(process.pid.get()) {
        Lookup::Found(facts) if facts.instance != process.instance => Err(process_gone(process)),
        Lookup::Gone => Err(process_gone(process)),
        Lookup::Found(facts) if world.stopped(process.pid.get()) => {
            Err(AdapterError::app_unresponsive(&listed_app_name(&facts))
                .with_disposition(DeliverySemantics::not_delivered()))
        }
        Lookup::Found(_) | Lookup::Unreadable => Ok(()),
    }
}

fn not_active(process: &ProcessIdentity, cause: &'static str) -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "The target application does not own the active window, so no key was sent",
    )
    .with_details(json!({
        "check": "window_active",
        "cause": cause,
        "pid": process.pid,
        "physical_delivery_started": false,
    }))
    .with_disposition(DeliverySemantics::not_delivered())
}

fn require_active(
    world: &impl KeyWorld,
    backend: &Backend,
    process: &ProcessIdentity,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    let pid = process.pid.get();
    if matches!(backend, Backend::X11(_)) && world.active_client_pid(deadline) != Some(pid) {
        return Err(not_active(process, "display_active_window"));
    }
    match world.focused_pid(deadline)? {
        Some(focused) if focused == pid => Ok(()),
        Some(_) => Err(not_active(process, "other_application")),
        None => Err(not_active(process, "frame_not_active")),
    }
}

fn after_delivery(
    world: &impl KeyWorld,
    process: &ProcessIdentity,
    result: ActionResult,
) -> Result<ActionResult, AdapterError> {
    require_process(world, process).map_err(|error| {
        error
            .with_details(json!({
                "pid": process.pid,
                "process_instance": process.instance,
                "physical_delivery_started": true,
            }))
            .with_disposition(DeliverySemantics::delivered_unverified())
    })?;
    Ok(result)
}

/// One chord for one process, with the two views of the desktop that decide
/// whether it may be sent.
struct Press<'a, P, K> {
    physical: &'a P,
    keys: &'a K,
    process: &'a ProcessIdentity,
    combo: &'a KeyCombo,
    policy: InteractionPolicy,
    deadline: Deadline,
}

fn press_in(
    request: Press<'_, impl PhysicalWorld, impl KeyWorld>,
) -> Result<ActionResult, AdapterError> {
    let Press {
        physical,
        keys,
        process,
        combo,
        policy,
        deadline,
    } = request;
    let backend = physical.backend()?;
    let mut gate = || {
        require_process(keys, process)?;
        require_active(keys, &backend, process, deadline)
    };
    let result = press_for_process(physical, combo, policy, &mut gate, deadline)?;
    after_delivery(keys, process, result)
}

pub(crate) fn press_key_for_app(
    bus: &SharedBus,
    process: ProcessIdentity,
    combo: &KeyCombo,
    policy: InteractionPolicy,
    lease: &InteractionLease,
) -> Result<ActionResult, AdapterError> {
    press_in(Press {
        physical: &LiveWorld { bus },
        keys: &LiveKeys { bus },
        process: &process,
        combo,
        policy,
        deadline: lease.deadline(),
    })
}

#[cfg(test)]
#[path = "key_dispatch_tests.rs"]
mod tests;
