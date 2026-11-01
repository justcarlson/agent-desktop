//! This process's hide holds while they last: for each holder, the thread
//! that took it, the renderer processes that acknowledged its `hide`, and
//! whether a renewal failed. A command takes its hold, reads receiver
//! evidence and sends its events on one thread, so the reads below see only
//! the calling thread's holds; the renewal thread marks a hold lost by its
//! nonce.

use std::sync::{Mutex, PoisonError};
use std::thread::ThreadId;

use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode};
use serde_json::json;

struct Hold {
    nonce: String,
    thread: ThreadId,
    renderers: Vec<u32>,
    lost: bool,
}

static HOLDS: Mutex<Vec<Hold>> = Mutex::new(Vec::new());

fn with_holds<T>(read: impl FnOnce(&mut Vec<Hold>) -> T) -> T {
    read(&mut HOLDS.lock().unwrap_or_else(PoisonError::into_inner))
}

fn mine(hold: &Hold) -> bool {
    hold.thread == std::thread::current().id()
}

/// Records, for the calling thread, that the renderers `pids` acknowledged
/// the holder `nonce`.
pub(crate) fn publish(nonce: &str, pids: Vec<u32>) {
    with_holds(|holds| {
        holds.retain(|hold| hold.nonce != nonce);
        holds.push(Hold {
            nonce: nonce.to_owned(),
            thread: std::thread::current().id(),
            renderers: pids,
            lost: false,
        });
    });
}

/// Records that the holder `nonce` could not be renewed.
pub(crate) fn mark_lost(nonce: &str) {
    with_holds(|holds| {
        for hold in holds.iter_mut().filter(|hold| hold.nonce == nonce) {
            hold.lost = true;
        }
    });
}

/// Forgets the holder `nonce`.
pub(crate) fn withdraw(nonce: &str) {
    with_holds(|holds| holds.retain(|hold| hold.nonce != nonce));
}

/// Every renderer process that acknowledged a holder the calling thread
/// holds.
pub(crate) fn acknowledged_renderers() -> Vec<u32> {
    with_holds(|holds| {
        holds
            .iter()
            .filter(|hold| mine(hold))
            .flat_map(|hold| hold.renderers.iter().copied())
            .collect()
    })
}

/// Whether the holder `nonce` is held and was not lost.
#[cfg(test)]
pub(crate) fn is_held(nonce: &str) -> bool {
    with_holds(|holds| holds.iter().any(|hold| hold.nonce == nonce && !hold.lost))
}

/// Refuses the next input event once a holder of the calling thread failed
/// to renew, so the work stops and releases what it holds before the holder
/// lapses.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn ensure_held() -> Result<(), AdapterError> {
    if !with_holds(|holds| holds.iter().any(|hold| mine(hold) && hold.lost)) {
        return Ok(());
    }
    Err(AdapterError::new(
        ErrorCode::ActionFailed,
        "The agent cursors could not stay hidden, so the input stopped",
    )
    .with_details(json!({"kind": "cursor_hide_renewal_failed"}))
    .with_disposition(DeliverySemantics::uncertain())
    .with_suggestion("Check the target's state before you retry"))
}

#[cfg(test)]
#[path = "hold_registry_tests.rs"]
mod tests;
