//! Cancellation for a Hyprland pointer or keyboard run. While the run holds
//! the input slot, SIGTERM, SIGINT and SIGHUP stop it at its next event; it
//! releases its held buttons or keys before the signal's own action is taken.

use std::sync::Arc;

use agent_desktop_core::{AdapterError, Deadline};

use crate::input::x11::progress::Progress;
use crate::input::x11::signal_release;
use crate::input::x11::worker_slot::WorkerSlot;

/// Runs `run` while it holds `slot`, with the signal release armed on its
/// progress. A slot still held by an earlier release refuses before `run`.
/// The slot is freed before the signal release is disarmed, so a signal taken
/// during `run` reaches its own action without waiting out the release wait.
pub(crate) fn with_release_on_signal<T>(
    slot: &Arc<WorkerSlot>,
    deadline: Deadline,
    run: impl FnOnce(&Progress) -> Result<T, AdapterError>,
) -> Result<T, AdapterError> {
    let held = slot.claim(deadline)?;
    let progress = Arc::new(Progress::default());
    let armed = signal_release::arm(Arc::clone(&progress), Arc::clone(slot));
    let outcome = run(&progress);
    drop(held);
    drop(armed);
    outcome
}

#[cfg(test)]
#[path = "pointer_cancel_tests.rs"]
mod tests;
