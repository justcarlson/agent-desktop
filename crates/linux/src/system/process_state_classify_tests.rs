use agent_desktop_core::process_state::ProcessState;

use super::{Probe, classify};
use crate::system::liveness::Liveness;

const EXITED: ProcessState = ProcessState::Exited { code: None };

#[test]
fn a_gone_process_is_exited_whatever_the_probes_say() {
    let sets: [&[Probe]; 5] = [
        &[],
        &[Probe::Answered],
        &[Probe::TimedOut],
        &[Probe::TimedOut, Probe::TimedOut],
        &[Probe::Unavailable],
    ];
    for probes in sets {
        for blocked in [false, true] {
            assert_eq!(classify(Liveness::Gone, probes, blocked), EXITED);
        }
    }
}

#[test]
fn a_stopped_process_that_did_not_answer_is_unresponsive() {
    for probes in [&[Probe::TimedOut][..], &[Probe::Unavailable], &[]] {
        assert_eq!(
            classify(Liveness::Stopped, probes, false),
            ProcessState::Unresponsive
        );
    }
}

#[test]
fn a_stopped_process_that_answered_is_running() {
    assert_eq!(
        classify(Liveness::Stopped, &[Probe::Answered], false),
        ProcessState::Running
    );
}

#[test]
fn a_live_process_that_answered_or_timed_out_once_is_running() {
    for probes in [
        &[Probe::Answered][..],
        &[Probe::TimedOut],
        &[Probe::Unavailable],
        &[],
    ] {
        assert_eq!(
            classify(Liveness::Alive, probes, true),
            ProcessState::Running
        );
    }
}

#[test]
fn two_timeouts_follow_the_blocked_loop_rule_both_ways() {
    let probes = [Probe::TimedOut, Probe::TimedOut];
    assert_eq!(
        classify(Liveness::Alive, &probes, true),
        ProcessState::Unresponsive
    );
    assert_eq!(
        classify(Liveness::Alive, &probes, false),
        ProcessState::Running
    );
}

#[test]
fn a_late_answer_after_a_timeout_is_running_under_the_rule() {
    assert_eq!(
        classify(Liveness::Alive, &[Probe::TimedOut, Probe::Answered], true),
        ProcessState::Running
    );
}

#[test]
fn the_shipped_rule_makes_a_live_process_with_two_timeouts_unresponsive() {
    let probes = [Probe::TimedOut, Probe::TimedOut];
    assert_eq!(
        classify(Liveness::Alive, &probes, super::BLOCKED_LOOP_RULE),
        ProcessState::Unresponsive
    );
}
