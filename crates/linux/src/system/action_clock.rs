use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::action_clock_store as store;

const WINDOW_MS: u64 = 2_000;
const WINDOW: Duration = Duration::from_millis(WINDOW_MS);

static LAST_ACTION: Mutex<Option<Instant>> = Mutex::new(None);

/// Records that this process has just delivered an action, so a listing of
/// surfaces right after it waits for a popup the action may have opened.
pub(crate) fn mark() {
    if let Ok(mut last) = LAST_ACTION.lock() {
        *last = Some(Instant::now());
    }
    store::record(None);
}

/// Records the same for the process the action reached, so a wait that
/// targets that process, in another invocation, can tell the action was its.
pub(crate) fn mark_process(pid: u32) {
    mark();
    store::record(Some(pid));
}

/// Records the action against the process it reached when the target carried
/// one, and against the session when it carried none.
pub(crate) fn mark_target(target: Option<u32>) {
    target.map_or_else(mark, mark_process);
}

/// Whether an action reached this process just now, in this or in another
/// invocation of the tool.
pub(crate) fn recent_for(pid: u32) -> bool {
    store::recent(Some(pid), WINDOW_MS)
}

/// Whether a listing for a window waits for a popup: only a focused window,
/// right after an action of this process.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the surface listing of the menu detector asks it; the wait asks recent_for"
    )
)]
pub(crate) fn settles(focused: bool) -> bool {
    LAST_ACTION
        .lock()
        .is_ok_and(|last| settles_at(focused, *last, Instant::now()))
        || (focused && store::recent(None, WINDOW_MS))
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
