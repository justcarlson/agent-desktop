use std::sync::Mutex;
use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_millis(2_000);

static LAST_ACTION: Mutex<Option<Instant>> = Mutex::new(None);

/// Records that this process has just delivered an action, so a listing of
/// surfaces right after it waits for a popup the action may have opened.
pub(crate) fn mark() {
    if let Ok(mut last) = LAST_ACTION.lock() {
        *last = Some(Instant::now());
    }
}

/// Whether a listing for a window waits for a popup: only a focused window,
/// right after an action of this process.
pub(crate) fn settles(focused: bool) -> bool {
    LAST_ACTION
        .lock()
        .is_ok_and(|last| settles_at(focused, *last, Instant::now()))
}

fn settles_at(focused: bool, last: Option<Instant>, now: Instant) -> bool {
    focused && within(last, now)
}

fn within(last: Option<Instant>, now: Instant) -> bool {
    last.is_some_and(|at| now.saturating_duration_since(at) < WINDOW)
}

#[cfg(test)]
#[path = "action_clock_tests.rs"]
mod tests;
