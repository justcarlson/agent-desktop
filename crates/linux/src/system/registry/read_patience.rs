//! How long a frame read waits for one application. A slice bounds a stalled
//! application; a live application that is drawing answers after its draw, so
//! it gets a grace on top of the slice before its read counts as unanswered.

use std::future::Future;
use std::time::Duration;

use crate::system::bus::bus_slice::within_slice;
use crate::system::process::proc_stat;

/// The wait for a read of one application: `slice`, then `grace` more on the
/// same call when the application's process was not stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReadPatience {
    pub(crate) slice: Duration,
    pub(crate) grace: Duration,
}

impl ReadPatience {
    /// `slice` with `grace` for a process that runs, and none for one that is
    /// stopped, which cannot answer however long it is waited for.
    pub(crate) fn for_state(slice: Duration, grace: Duration, stopped: bool) -> Self {
        Self {
            slice,
            grace: if stopped { Duration::ZERO } else { grace },
        }
    }

    pub(crate) fn for_process(slice: Duration, grace: Duration, pid: u32) -> Self {
        Self::for_state(slice, grace, proc_stat::is_stopped(pid))
    }

    /// Awaits `future` for the slice, then for the grace; `None` means it did
    /// not answer in either.
    pub(crate) async fn wait<T>(self, future: impl Future<Output = T>) -> Option<T> {
        within_slice(self.slice, future).await
    }
}

#[cfg(test)]
#[path = "read_patience_tests.rs"]
mod tests;
