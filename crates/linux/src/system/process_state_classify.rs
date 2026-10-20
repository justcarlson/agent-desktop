use agent_desktop_core::process_state::ProcessState;

use super::liveness::Liveness;

/// Whether two consecutive probe timeouts on a live process mean its main
/// loop is blocked. A blocked loop reads `S` in `/proc` and times out twice
/// running, while healthy applications never did, even with three busy
/// processes per core (`L31-17`, `L31-18`).
pub(super) const BLOCKED_LOOP_RULE: bool = true;

/// What the accessibility probe of a process answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Probe {
    Answered,
    TimedOut,
    Unavailable,
}

/// The one place that decides. `Gone` is `Exited`; a process the kernel
/// says is stopped is `Unresponsive` unless it answered; a live one is
/// `Running` unless it timed out twice and `blocked_loop` says that counts.
/// A single timeout is `Running`: core reads any error as "gone", so a slow
/// answer must not become one.
pub(super) fn classify(liveness: Liveness, probes: &[Probe], blocked_loop: bool) -> ProcessState {
    let answered = probes.contains(&Probe::Answered);
    let timeouts = probes.iter().filter(|p| **p == Probe::TimedOut).count();
    match liveness {
        Liveness::Gone => ProcessState::Exited { code: None },
        Liveness::Stopped if answered => ProcessState::Running,
        Liveness::Stopped => ProcessState::Unresponsive,
        Liveness::Alive if !answered && blocked_loop && timeouts >= 2 => ProcessState::Unresponsive,
        Liveness::Alive => ProcessState::Running,
    }
}

#[cfg(test)]
#[path = "process_state_classify_tests.rs"]
mod tests;
