use std::time::Duration;

use super::liveness::Liveness;

pub(super) const TERM_GRACE: Duration = Duration::from_millis(2_000);

/// What the close flow does next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Step {
    Done,
    Request,
    Unresponsive,
    Term,
    Kill,
    Wait,
}

/// The next step from the caller's wish, what the process is doing, and how
/// long ago a `SIGTERM` was delivered. A stopped process never gets
/// `SIGTERM`: the signal stays pending until it continues, so only
/// `SIGKILL` ends it. Without `force` nothing is signalled at all.
pub(super) fn next(
    force: bool,
    liveness: Liveness,
    since_term: Option<Duration>,
    grace: Duration,
) -> Step {
    match (liveness, force) {
        (Liveness::Gone, _) => Step::Done,
        (Liveness::Alive, false) => Step::Request,
        (Liveness::Stopped, false) => Step::Unresponsive,
        (Liveness::Stopped, true) => Step::Kill,
        (Liveness::Alive, true) => match since_term {
            None => Step::Term,
            Some(elapsed) if elapsed >= grace => Step::Kill,
            Some(_) => Step::Wait,
        },
    }
}

/// The grace between `SIGTERM` and `SIGKILL`: the measured grace, never more
/// than half of what is left of the budget, so `SIGKILL` and its wait still
/// fit.
pub(super) fn grace_within(remaining: Duration) -> Duration {
    TERM_GRACE.min(remaining / 2)
}

#[cfg(test)]
#[path = "close_plan_tests.rs"]
mod tests;
