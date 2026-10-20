use std::time::Duration;

use agent_desktop_core::{AdapterError, DeliverySemantics};

use super::Request;
use super::refusal;
use super::world::World;
use crate::system::close_plan::{Step, grace_within, next};
use crate::system::signal::{Sent, Signal};

struct Progress {
    term_at: Option<Duration>,
    grace: Duration,
    killed: bool,
}

impl Progress {
    fn sent_anything(&self) -> bool {
        self.term_at.is_some() || self.killed
    }
}

fn deliver(world: &impl World, signal: Signal, progress: &Progress) -> Result<Sent, AdapterError> {
    world.signal(signal).map_err(|error| {
        if progress.sent_anything() {
            error.with_disposition(DeliverySemantics::delivered_unverified())
        } else {
            error
        }
    })
}

fn expired(world: &impl World, request: &Request<'_>, progress: &Progress) -> AdapterError {
    let timeout = world.timeout_error();
    if progress.sent_anything() {
        refusal::waited_out(timeout, request, None)
    } else {
        timeout.with_disposition(DeliverySemantics::not_delivered())
    }
}

/// The close that signals: `SIGTERM`, a bounded grace, then `SIGKILL`; a
/// stopped process gets `SIGKILL` alone. `Ok` only once the process is
/// observed gone, and a process that is gone or replaced by another is the
/// benign outcome, never a signal to the stranger.
pub(super) fn close(world: &impl World, request: &Request<'_>) -> Result<bool, AdapterError> {
    let mut progress = Progress {
        term_at: None,
        grace: Duration::ZERO,
        killed: false,
    };
    loop {
        let since_term = progress
            .term_at
            .map(|at| world.elapsed().saturating_sub(at));
        match next(true, world.liveness(), since_term, progress.grace) {
            Step::Done => return Ok(progress.sent_anything()),
            Step::Term | Step::Kill if world.remaining().is_zero() && !progress.sent_anything() => {
                return Err(expired(world, request, &progress));
            }
            Step::Term => match deliver(world, Signal::Term, &progress)? {
                Sent::Delivered => {
                    progress.term_at = Some(world.elapsed());
                    progress.grace = grace_within(world.remaining());
                }
                Sent::AlreadyGone | Sent::Replaced => return Ok(progress.sent_anything()),
            },
            Step::Kill if !progress.killed => match deliver(world, Signal::Kill, &progress)? {
                Sent::Delivered => progress.killed = true,
                Sent::AlreadyGone | Sent::Replaced => return Ok(progress.sent_anything()),
            },
            Step::Kill | Step::Wait => {}
            Step::Request | Step::Unresponsive => {
                return Err(AdapterError::internal("a forced close planned a request"));
            }
        }
        if world.remaining().is_zero() {
            return Err(expired(world, request, &progress));
        }
        world.pause();
    }
}

#[cfg(test)]
#[path = "force_tests.rs"]
mod tests;
