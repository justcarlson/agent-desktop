use std::sync::mpsc::{Receiver, channel};
use std::thread;

use agent_desktop_core::Deadline;

use super::{X11Source, X11State};

/// A read that is already running on its own thread. A blocking round trip to
/// the display server ignores a deadline, so the caller waits for the answer
/// instead of making the call; a server that never answers leaves one parked
/// thread behind and the caller gets "unavailable".
pub(crate) struct PendingRead {
    receiver: Option<Receiver<X11State>>,
}

/// Starts reading at once, so the wait overlaps the caller's other work.
pub(crate) fn start(source: impl X11Source) -> PendingRead {
    let (sender, receiver) = channel();
    let spawned = thread::Builder::new()
        .name("x11-read".to_owned())
        .spawn(move || {
            let _ = sender.send(source.read());
        });
    PendingRead {
        receiver: spawned.ok().map(|_| receiver),
    }
}

impl PendingRead {
    /// The answer, or `Unavailable` when the deadline passes first or the read
    /// panicked.
    pub(crate) fn finish(self, deadline: Deadline) -> X11State {
        self.receiver
            .and_then(|receiver| receiver.recv_timeout(deadline.remaining()).ok())
            .unwrap_or(X11State::Unavailable)
    }
}

#[cfg(test)]
#[path = "bounded_tests.rs"]
mod tests;
