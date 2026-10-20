use agent_desktop_core::process_state::ProcessState;
use agent_desktop_core::{AdapterError, Deadline, ProcessIdentity};

use super::SharedBus;
use super::liveness::{Liveness, liveness};
use super::process_state_classify::{BLOCKED_LOOP_RULE, Probe, classify};
use super::registry::read_registry;

/// What `process_state` reads: the kernel's view of the process and one
/// accessibility probe. Split out so the order of the reads can be tested.
pub(super) trait Observer {
    fn liveness(&self) -> Liveness;
    fn probe(&self) -> Probe;
    fn expired(&self) -> bool;
}

struct Live<'a> {
    bus: &'a SharedBus,
    pid: u32,
    instance: &'a str,
    deadline: Deadline,
}

impl Observer for Live<'_> {
    fn liveness(&self) -> Liveness {
        liveness(self.pid, self.instance)
    }

    fn probe(&self) -> Probe {
        if self.deadline.is_expired() {
            return Probe::Unavailable;
        }
        let Ok(registry) = read_registry(self.bus, self.deadline) else {
            return Probe::Unavailable;
        };
        let mut found = registry
            .apps
            .iter()
            .filter(|app| app.root.pid() == self.pid)
            .peekable();
        if found.peek().is_none() {
            Probe::Unavailable
        } else if found.any(|app| app.answered) {
            Probe::Answered
        } else {
            Probe::TimedOut
        }
    }

    fn expired(&self) -> bool {
        self.deadline.is_expired()
    }
}

/// The liveness answer. It never returns an error: a probe that cannot run
/// leaves the kernel's answer standing, and the token is read again after
/// the probes so a process id recycled meanwhile reads as exited.
pub(super) fn observe(observer: &impl Observer, blocked_loop: bool) -> ProcessState {
    let first = observer.liveness();
    if first == Liveness::Gone {
        return classify(Liveness::Gone, &[], blocked_loop);
    }
    let mut probes = vec![observer.probe()];
    let timed_out = probes[0] == Probe::TimedOut;
    if blocked_loop && timed_out && first == Liveness::Alive && !observer.expired() {
        probes.push(observer.probe());
    }
    classify(observer.liveness(), &probes, blocked_loop)
}

pub(crate) fn process_state(
    bus: &SharedBus,
    process: ProcessIdentity,
    deadline: Deadline,
) -> Result<ProcessState, AdapterError> {
    let live = Live {
        bus,
        pid: process.pid.get(),
        instance: &process.instance,
        deadline,
    };
    Ok(observe(&live, BLOCKED_LOOP_RULE))
}

#[cfg(test)]
#[path = "process_state_tests.rs"]
mod tests;
