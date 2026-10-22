use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, ErrorCode, InteractionLease, InteractionPolicy};
use serde_json::json;

use super::world::ShellWorld;
use crate::system::shell::open::Opened;

const CLEANUP_MS: u64 = 2_000;
const CLOSED_POLL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Restore {
    Adopted,
    Closed,
    Unverified,
}

impl Restore {
    fn name(self) -> &'static str {
        match self {
            Self::Adopted => "adopted",
            Self::Closed => "closed",
            Self::Unverified => "unverified",
        }
    }
}

/// The message list for one mutation. A list that was showing on entry is
/// adopted and left showing; a list this session opened is closed on every
/// path out, and the close is checked by reading `SHOWING` back.
pub(crate) struct Session<'a> {
    world: &'a dyn ShellWorld,
    lease: &'a InteractionLease,
    opened: bool,
    finished: bool,
}

impl<'a> Session<'a> {
    pub(crate) fn open(
        world: &'a dyn ShellWorld,
        policy: InteractionPolicy,
        lease: &'a InteractionLease,
    ) -> Result<Self, AdapterError> {
        if !policy.is_headed() {
            return Err(headed_required(policy));
        }
        world.require_daemon(lease.deadline())?;
        match world.open(policy, lease) {
            Ok((_, opened)) => Ok(Self {
                world,
                lease,
                opened: opened == Opened::Opened,
                finished: false,
            }),
            Err(error) => {
                let mut cleanup = Self {
                    world,
                    lease,
                    opened: error
                        .details
                        .as_ref()
                        .and_then(|details| details.get("surface_opened_by_call"))
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false),
                    finished: false,
                };
                let restore = cleanup.restore();
                Err(with_restore(error, restore))
            }
        }
    }

    pub(crate) fn finish<T>(mut self, result: Result<T, AdapterError>) -> Result<T, AdapterError> {
        let restore = self.restore();
        match (result, restore) {
            (Ok(value), Restore::Adopted | Restore::Closed) => Ok(value),
            (Ok(_), Restore::Unverified) => Err(with_restore(restore_failed(), restore)),
            (Err(error), restore) => Err(with_restore(error, restore)),
        }
    }

    fn restore(&mut self) -> Restore {
        self.finished = true;
        if !self.opened {
            return Restore::Adopted;
        }
        let closed = close_within_own_budget(self.world, self.lease) && still_closed(self.world);
        if closed {
            Restore::Closed
        } else {
            Restore::Unverified
        }
    }
}

impl Drop for Session<'_> {
    fn drop(&mut self) {
        if !self.finished {
            self.restore();
        }
    }
}

/// The close runs on a deadline of its own: the lease deadline is spent when
/// the command waited out its whole budget, and a close that shared it would
/// leave the list open.
fn close_within_own_budget(world: &dyn ShellWorld, lease: &InteractionLease) -> bool {
    Deadline::detached_after(CLEANUP_MS)
        .and_then(|deadline| world.close(&lease.with_deadline(deadline)))
        .is_ok()
}

/// The shell's showing state can lag the click that closed the list, so the
/// read repeats until the list reads closed or the cleanup budget is spent.
fn still_closed(world: &dyn ShellWorld) -> bool {
    let Ok(deadline) = Deadline::detached_after(CLEANUP_MS) else {
        return false;
    };
    loop {
        match world.is_showing(deadline) {
            Ok(false) => return true,
            Err(_) => return false,
            Ok(true) if deadline.is_expired() => return false,
            Ok(true) => std::thread::sleep(CLOSED_POLL),
        }
    }
}

fn headed_required(policy: InteractionPolicy) -> AdapterError {
    AdapterError::policy_denied_for_policy(
        "Changing notifications moves the pointer on the shell and needs headed mode",
        policy,
    )
    .with_suggestion("Pass --headed to let the command click in the message list")
}

fn restore_failed() -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "The call finished but the message list it opened could not be closed",
    )
    .with_suggestion("Close the message list by hand with Escape or a click on the date menu")
}

fn with_restore(mut error: AdapterError, restore: Restore) -> AdapterError {
    let mut details = match error.details.take() {
        Some(serde_json::Value::Object(map)) => map,
        _ => serde_json::Map::new(),
    };
    details.insert("list_restore".to_owned(), json!(restore.name()));
    error.with_details(serde_json::Value::Object(details))
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "session_lease_tests.rs"]
mod lease_tests;
