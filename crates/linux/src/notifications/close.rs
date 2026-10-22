use std::time::Duration;

use agent_desktop_core::{AdapterError, InteractionLease, InteractionPolicy, MouseButton};

use super::extract::Entry;
use super::seen::{Seen, holds};
use super::target::missing_control;
use super::verify::{REMOVAL_SETTLE, bystander_error, bystanders, settle_read, survived_error};
use super::world::ShellWorld;

const PRESS_SETTLE: Duration = Duration::from_millis(2_500);
const MAX_PRESSES: usize = 2;

/// Why an entry was not closed, split by what the caller does with it: a
/// press that never went out, a list unreadable afterwards, an entry that
/// stayed, or another entry that left with it.
pub(crate) enum Unclosed {
    Press(AdapterError),
    Unread(AdapterError),
    Survived,
    Bystander,
}

impl Unclosed {
    pub(crate) fn into_error(self) -> AdapterError {
        match self {
            Self::Press(error) | Self::Unread(error) => error,
            Self::Survived => survived_error("close control of the notification"),
            Self::Bystander => bystander_error(),
        }
    }
}

/// Closes one entry and proves it by its effect: the entry's own object must
/// leave the list, and no other, and another entry leaving ends the attempt as
/// a failure. The header's buttons are pressed last one first; a button that
/// only expands the entry leaves its object standing, so the next one is
/// pressed. The entry still standing after every press is a failure, never a
/// success.
pub(crate) fn close_entry(
    world: &dyn ShellWorld,
    entry: &Entry,
    before: &[Seen],
    policy: InteractionPolicy,
    lease: &InteractionLease,
) -> Result<Vec<Seen>, Unclosed> {
    let target = Seen {
        info: entry.info.clone(),
        node: entry.node.clone(),
    };
    let deadline = lease.deadline();
    let candidates: Vec<_> = entry.close_candidates().take(MAX_PRESSES).collect();
    if candidates.is_empty() {
        return Err(Unclosed::Press(missing_control(
            "close button",
            entry.info.index,
        )));
    }
    let count = candidates.len();
    for (position, control) in candidates.into_iter().enumerate() {
        world
            .click(&control.element, MouseButton::Left, policy, lease)
            .map_err(Unclosed::Press)?;
        let window = if position + 1 == count {
            REMOVAL_SETTLE
        } else {
            PRESS_SETTLE
        };
        let after = settle_read(world, deadline, window, |now| !holds(now, &target.node))
            .map_err(Unclosed::Unread)?;
        if !bystanders(before, &after, &target).is_empty() {
            return Err(Unclosed::Bystander);
        }
        if !holds(&after, &target.node) {
            return Ok(after);
        }
    }
    Err(Unclosed::Survived)
}

#[cfg(test)]
#[path = "close_tests.rs"]
mod tests;
