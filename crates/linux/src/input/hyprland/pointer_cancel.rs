//! Cancellation for a Hyprland pointer run. While the run holds the input
//! slot, SIGTERM, SIGINT and SIGHUP stop it at its next event; it releases
//! its held buttons before the signal's own action is taken.

use std::sync::Arc;

use agent_desktop_core::{AdapterError, Deadline};

use crate::input::x11::progress::Progress;
use crate::input::x11::signal_release;
use crate::input::x11::worker_slot::WorkerSlot;

/// Runs `run` while it holds `slot`, with the signal release armed on its
/// progress. A slot still held by an earlier release refuses before `run`.
pub(crate) fn with_release_on_signal<T>(
    slot: &Arc<WorkerSlot>,
    deadline: Deadline,
    run: impl FnOnce(&Progress) -> Result<T, AdapterError>,
) -> Result<T, AdapterError> {
    let _held = slot.claim(deadline)?;
    let progress = Arc::new(Progress::default());
    let _armed = signal_release::arm(Arc::clone(&progress), Arc::clone(slot));
    run(&progress)
}

#[cfg(test)]
#[path = "pointer_cancel_tests.rs"]
mod tests;
