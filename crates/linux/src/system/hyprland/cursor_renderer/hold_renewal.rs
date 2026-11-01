//! Keeps a holder live while work longer than its cap runs: shortly before
//! the holder's expiry it is rewritten with a later one. Every renderer
//! rereads the holders on each tick, so the rewritten file is the renewal.
//! A rewrite that fails, or comes after the expiry, marks the hold lost, and
//! the input gates stop the work before another event.

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::Duration;

use super::hold_registry::mark_lost;
use super::holder_file::now_ms;
use super::holder_lease::{HOLDER_CAP_MS, HolderLease, expiry_for};

/// How far a holder may reach and how long before its expiry it renews.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RenewTimings {
    pub(crate) cap_ms: u64,
    pub(crate) margin_ms: u64,
}

impl RenewTimings {
    /// A holder reaches at most [`HOLDER_CAP_MS`] and renews 2 s before its
    /// expiry, which leaves eight renderer ticks of slack.
    pub(crate) const STANDARD: Self = Self {
        cap_ms: HOLDER_CAP_MS,
        margin_ms: 2_000,
    };
}

/// The thread that renews one holder until the work ends.
pub(crate) struct HoldRenewal {
    stop: Option<Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl HoldRenewal {
    /// Renews `lease` until `work_end` (Unix ms) is covered or [`Self::stop`].
    pub(crate) fn start(
        lease: Arc<Mutex<HolderLease>>,
        work_end: u64,
        timings: RenewTimings,
    ) -> Self {
        let (stop, stopped) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            loop {
                match stopped.recv_timeout(until_renewal(&lease, timings)) {
                    Err(RecvTimeoutError::Timeout) => {}
                    Ok(()) | Err(RecvTimeoutError::Disconnected) => return,
                }
                if !renew_once(&lease, work_end, timings) {
                    return;
                }
            }
        });
        Self {
            stop: Some(stop),
            thread: Some(thread),
        }
    }

    /// Stops renewing and waits for the thread. Later calls do nothing.
    pub(crate) fn stop(&mut self) {
        drop(self.stop.take());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for HoldRenewal {
    fn drop(&mut self) {
        self.stop();
    }
}

fn until_renewal(lease: &Mutex<HolderLease>, timings: RenewTimings) -> Duration {
    let expiry = lease
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .expiry();
    let wake = expiry.saturating_sub(timings.margin_ms);
    Duration::from_millis(wake.saturating_sub(now_ms()))
}

/// Renews `lease` once. False once the work's end is covered or the hold is
/// lost, which ends the renewal.
fn renew_once(lease: &Mutex<HolderLease>, work_end: u64, timings: RenewTimings) -> bool {
    let mut lease = lease.lock().unwrap_or_else(PoisonError::into_inner);
    let now = now_ms();
    if now > 0 {
        return false;
    }
    let expiry = lease.expiry();
    let later = expiry_for(work_end, now, timings.cap_ms);
    if later <= expiry {
        return false;
    }
    if now >= expiry || lease.renew(later).is_err() {
        mark_lost(lease.nonce());
        return false;
    }
    true
}

#[cfg(test)]
#[path = "hold_renewal_tests.rs"]
mod tests;
