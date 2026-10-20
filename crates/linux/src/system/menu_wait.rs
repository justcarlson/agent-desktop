use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode, ProcessIdentity};

use super::SharedBus;
use super::action_clock;
use super::liveness::{Liveness, liveness};
use super::menu_anchor::menu_open;
use super::process_facts::{self, ProcessFacts};

const POLL_INTERVAL: Duration = Duration::from_millis(50);

pub(super) trait MenuWorld {
    fn process(&self) -> &ProcessIdentity;
    fn liveness(&self) -> Liveness;
    fn menu_open(&self, deadline: Deadline) -> Result<bool, AdapterError>;
    fn pause(&self, gap: Duration);
    fn recent_action(&self) -> bool;

    fn label(&self) -> String {
        let pid = self.process().pid.get();
        app_label(process_facts::read(pid).as_ref(), pid)
    }
}

fn app_label(facts: Option<&ProcessFacts>, pid: u32) -> String {
    facts
        .and_then(|facts| facts.exe_basename.clone().or(Some(facts.comm.clone())))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| format!("pid {pid}"))
}

struct BusWorld<'a> {
    bus: &'a SharedBus,
    process: ProcessIdentity,
}

impl MenuWorld for BusWorld<'_> {
    fn process(&self) -> &ProcessIdentity {
        &self.process
    }

    fn liveness(&self) -> Liveness {
        liveness(self.process.pid.get(), &self.process.instance)
    }

    fn menu_open(&self, deadline: Deadline) -> Result<bool, AdapterError> {
        menu_open(self.bus, &self.process, deadline)
    }

    fn pause(&self, gap: Duration) {
        std::thread::sleep(gap);
    }

    fn recent_action(&self) -> bool {
        action_clock::recent_for(self.process.pid.get())
    }
}

fn direction_message(open: bool) -> &'static str {
    if open {
        "No menu opened before the deadline"
    } else {
        "Menu did not close before the deadline"
    }
}

fn expiry_detail(open: bool, state_never_read: bool) -> String {
    let direction = direction_message(open);
    if state_never_read {
        format!("{direction}; no poll read the menu state before the deadline")
    } else {
        direction.to_owned()
    }
}

fn stale_process_error(process: &ProcessIdentity) -> AdapterError {
    AdapterError::new(
        ErrorCode::StaleRef,
        "Target process instance is no longer running",
    )
    .with_suggestion(concat!(
        "The application exited during the wait. Re-resolve it with list-apps ",
        "and retry against the new process instance, or treat its exit as the outcome.",
    ))
    .with_disposition(DeliverySemantics::not_delivered())
    .with_details(serde_json::json!({
        "pid": process.pid.get(),
        "process_instance": process.instance,
    }))
}

fn verify_process(world: &impl MenuWorld) -> Result<(), AdapterError> {
    match world.liveness() {
        Liveness::Alive => Ok(()),
        Liveness::Gone => Err(stale_process_error(world.process())),
        Liveness::Stopped => Err(AdapterError::app_unresponsive(&world.label())),
    }
}

/// Whether a read that matches the wanted state may end the wait. A menu
/// that an action of this process has just asked to open stays closed for
/// the first few hundred milliseconds, so a closed read right after such an
/// action proves nothing until the menu has been seen open or the settle
/// window of the action has passed.
fn settled(open: bool, seen_open: bool, world: &impl MenuWorld) -> bool {
    open || seen_open || !world.recent_action()
}

pub(super) fn run(
    world: &impl MenuWorld,
    open: bool,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    let mut state_never_read = true;
    let mut seen_open = false;
    loop {
        verify_process(world)?;
        match world.menu_open(deadline) {
            Ok(state) if state == open && settled(open, seen_open, world) => {
                verify_process(world)?;
                return Ok(());
            }
            Ok(state) => {
                state_never_read = false;
                seen_open |= state;
            }
            Err(error) if error.code == ErrorCode::Timeout => {}
            Err(error) => {
                verify_process(world)?;
                return Err(error);
            }
        }
        if deadline.is_expired() {
            return Err(deadline
                .timeout_error()
                .with_platform_detail(expiry_detail(open, state_never_read)));
        }
        let gap = deadline
            .remaining_slice(POLL_INTERVAL)
            .map_err(|error| error.with_platform_detail(direction_message(open)))?;
        world.pause(gap);
    }
}

pub(crate) fn wait_for_menu(
    bus: &SharedBus,
    process: ProcessIdentity,
    open: bool,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    run(&BusWorld { bus, process }, open, deadline)
}

#[cfg(test)]
#[path = "menu_wait_tests.rs"]
mod tests;
