use std::time::Duration;

use agent_desktop_core::Deadline;

use super::held_journal::HeldJournal;
use super::worker_slot;
use super::xtest_sink::XtestSink;
use crate::system::display_server::{DisplayServer, session_server};

const SWEEP_BUDGET_MS: u64 = 1_000;
const CUTOFF: Duration = Duration::from_millis(1_500);

/// Releases the keys and buttons a dead input call of this display left held.
/// A command that only reads the desktop calls it, so a killed drag does not
/// stay stuck until the next physical input. It connects only when a record
/// of a dead process exists.
pub(crate) fn sweep() {
    let DisplayServer::X11(display) = session_server() else {
        return;
    };
    let Some(journal) = HeldJournal::in_runtime_dir(&display) else {
        return;
    };
    if !journal.has_stale_entries() {
        return;
    }
    let Ok(deadline) = Deadline::detached_after(SWEEP_BUDGET_MS) else {
        return;
    };
    let Ok(_claim) = worker_slot::shared().claim(deadline) else {
        return;
    };
    let pending = crate::system::x11::start(move || {
        if let Ok(mut sink) = XtestSink::connect(&display, deadline, CUTOFF) {
            journal.release_readable_stale(&mut sink);
        }
    });
    let _ = pending.finish(deadline);
}
