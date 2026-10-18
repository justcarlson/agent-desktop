use std::sync::mpsc::{Receiver, channel};
use std::thread;

use agent_desktop_core::Deadline;

/// A display-server read running on its own thread: X11 property and RandR
/// replies are synchronous round trips to a display server that cannot be
/// allowed to stall the caller, so the caller bounds every read by
/// `Deadline`.
pub(crate) struct PendingRead<T> {
    receiver: Option<Receiver<T>>,
}

/// Starts one read on a dedicated thread. When the thread cannot even be
/// spawned the pending answer reads as absent.
pub(crate) fn start<T: Send + 'static>(
    read: impl FnOnce() -> T + Send + 'static,
) -> PendingRead<T> {
    let (sender, receiver) = channel();
    let spawned = thread::Builder::new()
        .name("x11-read".to_owned())
        .spawn(move || {
            let _ = sender.send(read());
        });
    PendingRead {
        receiver: spawned.ok().map(|_| receiver),
    }
}

impl<T> PendingRead<T> {
    /// The answer, or `None` when the deadline passes first, the thread
    /// panicked, or the thread was never spawned.
    pub(crate) fn finish(self, deadline: Deadline) -> Option<T> {
        self.receiver
            .and_then(|receiver| receiver.recv_timeout(deadline.remaining()).ok())
    }
}

#[cfg(test)]
#[path = "bounded_tests.rs"]
mod tests;
