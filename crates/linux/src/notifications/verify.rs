use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode, NotificationInfo};

use super::list::read_listing;
use super::signals::Signal;
use super::world::ShellWorld;

const POLL_INTERVAL: Duration = Duration::from_millis(100);
pub(crate) const REMOVAL_SETTLE: Duration = Duration::from_millis(6_000);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Removal {
    Pending,
    Settled,
    Bystander,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Evidence {
    ActionInvoked,
    EntryRemoved,
    SettleElapsed,
}

impl Evidence {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::ActionInvoked => "action_invoked",
            Self::EntryRemoved => "entry_removed",
            Self::SettleElapsed => "settle_elapsed",
        }
    }
}

/// The identity a mutation verifies against: source application, title and body.
pub(crate) fn same_identity(left: &NotificationInfo, right: &NotificationInfo) -> bool {
    left.app_name == right.app_name && left.title == right.title && left.body == right.body
}

pub(crate) fn matching_count(entries: &[NotificationInfo], target: &NotificationInfo) -> usize {
    entries
        .iter()
        .filter(|entry| same_identity(entry, target))
        .count()
}

/// The members of `before` that `after` no longer holds, matched one for one
/// by identity so two identical entries lose only the one that left.
pub(crate) fn lost(
    before: &[NotificationInfo],
    after: &[NotificationInfo],
) -> Vec<NotificationInfo> {
    matched(before, after, false)
}

fn matched(
    source: &[NotificationInfo],
    candidates: &[NotificationInfo],
    present: bool,
) -> Vec<NotificationInfo> {
    let mut unmatched: Vec<&NotificationInfo> = candidates.iter().collect();
    source
        .iter()
        .filter(|member| {
            let position = unmatched
                .iter()
                .position(|candidate| same_identity(candidate, member));
            if let Some(position) = position {
                unmatched.swap_remove(position);
            }
            position.is_some() == present
        })
        .cloned()
        .collect()
}

/// A dismissal is settled only when exactly the target left. Nothing gone is
/// still pending; anything else gone, with or without the target, is a bystander.
pub(crate) fn judge_removal(
    before: &[NotificationInfo],
    after: &[NotificationInfo],
    target: &NotificationInfo,
) -> Removal {
    let gone = lost(before, after);
    match gone.as_slice() {
        [] => Removal::Pending,
        [only] if same_identity(only, target) => Removal::Settled,
        _ => Removal::Bystander,
    }
}

/// The members of the captured set that the current list still holds,
/// matched one for one by identity.
pub(crate) fn survivors(
    captured: &[NotificationInfo],
    current: &[NotificationInfo],
) -> Vec<NotificationInfo> {
    matched(captured, current, true)
}

pub(crate) fn survivor_line(index: usize) -> String {
    format!("#{index}: notification from the captured set is still present")
}

/// An action signal counts only when it is the single one seen in the window
/// and the list copy shows the entry gone. No id is held, so a second signal
/// in the window cannot be told from the clicked entry's own and ends the claim.
pub(crate) fn evidence(signals: &[Signal], entry_gone: bool) -> Evidence {
    let invoked = signals
        .iter()
        .filter(|signal| matches!(signal, Signal::ActionInvoked(_)))
        .count();
    match (entry_gone, invoked) {
        (true, 1) => Evidence::ActionInvoked,
        (true, _) => Evidence::EntryRemoved,
        (false, _) => Evidence::SettleElapsed,
    }
}

/// Re-reads the list until `done` holds or the settle window closes, and
/// returns the last listing either way. An expired window is an answer, not
/// an error: the caller decides what survival means.
pub(crate) fn settle_read(
    world: &dyn ShellWorld,
    deadline: Deadline,
    done: impl Fn(&[NotificationInfo]) -> bool,
) -> Result<Vec<NotificationInfo>, AdapterError> {
    let settle = deadline.capped(REMOVAL_SETTLE);
    let mut current = read_listing(world, settle)
        .map_err(unconfirmed_read)?
        .infos();
    loop {
        if done(&current) || settle.is_expired() {
            return Ok(current);
        }
        std::thread::sleep(settle.remaining_slice(POLL_INTERVAL)?);
        current = match read_listing(world, settle) {
            Ok(listing) => listing.infos(),
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
        format!("The {what} was clicked but the entry is still in the message list"),
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

#[cfg(test)]
#[path = "verify_tests.rs"]
mod tests;
