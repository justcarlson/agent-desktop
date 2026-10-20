use std::cell::{Cell, RefCell};

use agent_desktop_core::process_state::ProcessState;

use super::{Observer, observe};
use crate::system::liveness::Liveness;
use crate::system::process_state_classify::Probe;

struct Fake {
    liveness: RefCell<Vec<Liveness>>,
    probes: RefCell<Vec<Probe>>,
    probed: Cell<u32>,
    expired: bool,
}

fn fake(liveness: &[Liveness], probes: &[Probe]) -> Fake {
    let mut l = liveness.to_vec();
    l.reverse();
    let mut p = probes.to_vec();
    p.reverse();
    Fake {
        liveness: RefCell::new(l),
        probes: RefCell::new(p),
        probed: Cell::new(0),
        expired: false,
    }
}

impl Observer for Fake {
    fn liveness(&self) -> Liveness {
        self.liveness.borrow_mut().pop().expect("liveness read")
    }

    fn probe(&self) -> Probe {
        self.probed.set(self.probed.get() + 1);
        self.probes.borrow_mut().pop().expect("probe read")
    }

    fn expired(&self) -> bool {
        self.expired
    }
}

#[test]
fn a_gone_process_is_exited_without_probing() {
    let observer = fake(&[Liveness::Gone], &[]);
    assert_eq!(
        observe(&observer, false),
        ProcessState::Exited { code: None }
    );
    assert_eq!(observer.probed.get(), 0);
}

#[test]
fn a_token_that_changed_during_the_probe_is_exited() {
    let observer = fake(&[Liveness::Alive, Liveness::Gone], &[Probe::Answered]);
    assert_eq!(
        observe(&observer, false),
        ProcessState::Exited { code: None }
    );
}

#[test]
fn a_stopped_process_that_cannot_be_probed_is_unresponsive_not_an_error() {
    let observer = fake(
        &[Liveness::Stopped, Liveness::Stopped],
        &[Probe::Unavailable],
    );
    assert_eq!(observe(&observer, false), ProcessState::Unresponsive);
}

#[test]
fn a_probe_timeout_on_a_live_process_is_running() {
    let observer = fake(&[Liveness::Alive, Liveness::Alive], &[Probe::TimedOut]);
    assert_eq!(observe(&observer, false), ProcessState::Running);
    assert_eq!(observer.probed.get(), 1);
}

#[test]
fn the_second_probe_runs_only_when_the_blocked_loop_rule_is_on() {
    let on = fake(
        &[Liveness::Alive, Liveness::Alive],
        &[Probe::TimedOut, Probe::TimedOut],
    );
    assert_eq!(observe(&on, true), ProcessState::Unresponsive);
    assert_eq!(on.probed.get(), 2);
    let off = fake(&[Liveness::Alive, Liveness::Alive], &[Probe::TimedOut]);
    assert_eq!(observe(&off, false), ProcessState::Running);
    assert_eq!(off.probed.get(), 1);
}

#[test]
fn an_exhausted_deadline_skips_the_second_probe_and_still_answers() {
    let mut observer = fake(&[Liveness::Alive, Liveness::Alive], &[Probe::TimedOut]);
    observer.expired = true;
    assert_eq!(observe(&observer, true), ProcessState::Running);
    assert_eq!(observer.probed.get(), 1);
}

#[test]
fn a_stopped_process_is_decided_by_the_kernel_and_probed_once() {
    let observer = fake(&[Liveness::Stopped, Liveness::Stopped], &[Probe::TimedOut]);
    assert_eq!(observe(&observer, true), ProcessState::Unresponsive);
    assert_eq!(observer.probed.get(), 1);
}
