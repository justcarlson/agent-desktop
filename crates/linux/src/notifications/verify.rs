use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};

use super::list::read_listing;
use super::seen::{Seen, holds, same_object};
use super::signals::Signal;
use super::world::ShellWorld;

const POLL_INTERVAL: Duration = Duration::from_millis(100);
pub(crate) const REMOVAL_SETTLE: Duration = Duration::from_millis(6_000);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Evidence {
    ActionInvoked,
    EntryRemoved,
}

impl Evidence {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::ActionInvoked => "action_invoked",
            Self::EntryRemoved => "entry_removed",
        }
    }
}

/// The identity a mutation verifies against: source application, title and body.
pub(crate) fn same_identity(
    left: &agent_desktop_core::NotificationInfo,
    right: &agent_desktop_core::NotificationInfo,
) -> bool {
    left.app_name == right.app_name && left.title == right.title && left.body == right.body
}

/// The entries of `before`, other than the target, that `after` no longer
/// holds: notifications that left while only the target should have.
pub(crate) fn bystanders<'a>(before: &'a [Seen], after: &[Seen], target: &Seen) -> Vec<&'a Seen> {
    before
        .iter()
        .filter(|seen| !same_object(&seen.node, &target.node) && !holds(after, &seen.node))
        .collect()
}

/// The captured entries whose object the current list no longer holds.
pub(crate) fn departed<'a>(captured: &'a [Seen], current: &[Seen]) -> Vec<&'a Seen> {
    captured
        .iter()
        .filter(|seen| !holds(current, &seen.node))
        .collect()
}

/// The captured entries whose object the current list still holds.
pub(crate) fn remaining<'a>(captured: &'a [Seen], current: &[Seen]) -> Vec<&'a Seen> {
    captured
        .iter()
        .filter(|seen| holds(current, &seen.node))
        .collect()
}

pub(crate) fn survivor_line(index: usize) -> String {
    format!("#{index}: notification from the captured set is still present")
}

/// What proves an action took effect. The entry leaving the list is the
/// proof; an action signal only sharpens it, and only when it is the single
/// one seen in the window, because no id is held to tie a signal to the click.
pub(crate) fn evidence(signals: &[Signal], target_gone: bool) -> Option<Evidence> {
    if !target_gone {
        return None;
    }
    let invoked = signals
        .iter()
        .filter(|signal| matches!(signal, Signal::ActionInvoked(_)))
        .count();
    Some(if invoked == 1 {
        Evidence::ActionInvoked
    } else {
        Evidence::EntryRemoved
    })
}

/// Re-reads the list until `done` holds or the settle window closes, and
/// returns the last listing either way. An expired window is an answer, not
/// an error: the caller decides what survival means.
pub(crate) fn settle_read(
    world: &dyn ShellWorld,
    deadline: Deadline,
    window: Duration,
    done: impl Fn(&[Seen]) -> bool,
) -> Result<Vec<Seen>, AdapterError> {
    let settle = deadline.capped(window);
    let mut current = read_listing(world, settle)
        .map_err(unconfirmed_read)?
        .seen();
    loop {
        if done(&current) || settle.is_expired() {
            return Ok(current);
        }
        std::thread::sleep(settle.remaining_slice(POLL_INTERVAL)?);
        current = match read_listing(world, settle) {
            Ok(listing) => listing.seen(),
            Err(error) if error.code == ErrorCode::Timeout => return Ok(current),
            Err(error) => return Err(unconfirmed_read(error)),
        };
    }
}

/// The click went out, so a list that cannot be read afterwards leaves the
/// delivery in doubt and never turns it into a bare failure of the read.
pub(crate) fn unconfirmed_read(error: AdapterError) -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "The click was delivered but the message list could not be read afterwards",
    )
    .with_suggestion("Run list-notifications to see what the message list holds now")
    .with_platform_detail(error.message)
    .with_disposition(DeliverySemantics::delivered_unverified())
}

pub(crate) fn survived_error(what: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        format!("The {what} was pressed but the notification is still in the message list"),
    )
    .with_suggestion("Run list-notifications and retry if the entry is still there")
    .with_disposition(DeliverySemantics::delivered_unverified())
}

pub(crate) fn bystander_error() -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "A notification other than the target left the message list during the dismissal",
    )
    .with_suggestion("Run list-notifications to see what the message list holds now")
    .with_disposition(DeliverySemantics::delivered_unverified())
}

pub(crate) fn unproven_action_error(name: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        format!("The '{name}' button was pressed but the notification stayed in the message list"),
    )
    .with_suggestion("Run list-notifications; a notification that stays may keep its actions")
    .with_disposition(DeliverySemantics::delivered_unverified())
}

#[cfg(test)]
#[path = "verify_tests.rs"]
mod tests;
