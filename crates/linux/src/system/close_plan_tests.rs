use std::time::Duration;

use super::{Step, TERM_GRACE, grace_within, next};
use crate::system::liveness::Liveness;

const GRACE: Duration = Duration::from_millis(100);

#[test]
fn a_live_process_without_force_gets_close_requests() {
    assert_eq!(next(false, Liveness::Alive, None, GRACE), Step::Request);
}

#[test]
fn a_stopped_process_without_force_is_unresponsive() {
    assert_eq!(
        next(false, Liveness::Stopped, None, GRACE),
        Step::Unresponsive
    );
}

#[test]
fn a_live_process_with_force_gets_term_first() {
    assert_eq!(next(true, Liveness::Alive, None, GRACE), Step::Term);
}

#[test]
fn a_stopped_process_with_force_skips_term_for_kill() {
    assert_eq!(next(true, Liveness::Stopped, None, GRACE), Step::Kill);
}

#[test]
fn term_inside_the_grace_waits_and_past_the_grace_kills() {
    let inside = Some(GRACE - Duration::from_millis(1));
    assert_eq!(next(true, Liveness::Alive, inside, GRACE), Step::Wait);
    assert_eq!(next(true, Liveness::Alive, Some(GRACE), GRACE), Step::Kill);
}

#[test]
fn a_gone_process_is_done_in_every_mode() {
    for force in [false, true] {
        for since in [None, Some(GRACE)] {
            assert_eq!(next(force, Liveness::Gone, since, GRACE), Step::Done);
        }
    }
}

#[test]
fn no_graceful_step_is_ever_a_signal() {
    for liveness in [Liveness::Alive, Liveness::Stopped, Liveness::Gone] {
        for since in [None, Some(GRACE), Some(Duration::ZERO)] {
            let step = next(false, liveness, since, GRACE);
            assert!(
                !matches!(step, Step::Term | Step::Kill),
                "{liveness:?} {since:?}"
            );
        }
    }
}

#[test]
fn the_grace_never_takes_more_than_half_the_budget() {
    assert_eq!(grace_within(Duration::from_secs(60)), TERM_GRACE);
    assert_eq!(
        grace_within(Duration::from_millis(1_000)),
        Duration::from_millis(500)
    );
    assert_eq!(grace_within(Duration::ZERO), Duration::ZERO);
}
