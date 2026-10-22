use agent_desktop_core::{
    AdapterError, Deadline, DismissAllNotificationsRequest, InteractionLease, NotificationFilter,
    NotificationInfo,
};

use super::close::{Unclosed, close_entry};
use super::list::{apply_filter, read_listing};
use super::seen::{Seen, same_object};
use super::session::Session;
use super::verify::{bystander_error, departed, remaining, survivor_line, unconfirmed_read};
use super::world::{LiveWorld, ShellWorld};
use crate::system::SharedBus;

type Outcome = (Vec<NotificationInfo>, Vec<String>);

pub(crate) fn dismiss_all(
    bus: &SharedBus,
    request: DismissAllNotificationsRequest<'_>,
    lease: &InteractionLease,
) -> Result<Outcome, AdapterError> {
    dismiss_all_in(&LiveWorld::new(bus), request, lease)
}

/// Clears the list, or only the entries of one app, one entry at a time: each
/// entry is closed through its own header control and only counts as
/// dismissed when its own object has left the list. No button that acts on
/// the whole list is pressed, because none can be told apart by anything but
/// its place. The outcome is judged against the set captured first.
pub(crate) fn dismiss_all_in(
    world: &dyn ShellWorld,
    request: DismissAllNotificationsRequest<'_>,
    lease: &InteractionLease,
) -> Result<Outcome, AdapterError> {
    if request.policy.is_headed() && nothing_to_clear(world, &request, lease.deadline())? {
        return Ok((Vec::new(), Vec::new()));
    }
    let session = Session::open(world, request.policy, lease)?;
    let result = run(world, &request, lease);
    session.finish(result)
}

/// Reads the closed list first: an empty list, or an app filter that matches
/// nothing, needs no click and no open, and the open is the step that fails
/// on a list that has just emptied.
fn nothing_to_clear(
    world: &dyn ShellWorld,
    request: &DismissAllNotificationsRequest<'_>,
    deadline: Deadline,
) -> Result<bool, AdapterError> {
    let listing = read_listing(world, deadline)?;
    Ok(apply_filter(listing.infos(), &app_scope(request)).is_empty())
}

fn app_scope(request: &DismissAllNotificationsRequest<'_>) -> NotificationFilter {
    NotificationFilter {
        app: request.app_filter.map(str::to_owned),
        ..NotificationFilter::default()
    }
}

fn run(
    world: &dyn ShellWorld,
    request: &DismissAllNotificationsRequest<'_>,
    lease: &InteractionLease,
) -> Result<Outcome, AdapterError> {
    let deadline = lease.deadline();
    let listing = read_listing(world, deadline)?;
    let wanted: Vec<usize> = apply_filter(listing.infos(), &app_scope(request))
        .iter()
        .map(|info| info.index)
        .collect();
    let captured: Vec<Seen> = listing
        .seen()
        .into_iter()
        .filter(|seen| wanted.contains(&seen.info.index))
        .collect();
    if captured.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let mut recorded: Vec<(usize, String)> = Vec::new();
    for target in &captured {
        let now = read_listing(world, deadline)?;
        let before = now.seen();
        let Some(entry) = now
            .entries
            .iter()
            .find(|entry| same_object(&entry.node, &target.node))
        else {
            continue;
        };
        match close_entry(world, entry, &before, request.policy, lease) {
            Ok(_) => {}
            Err(Unclosed::Unread(error)) => return Err(error),
            Err(Unclosed::Press(error)) => recorded.push((target.info.index, error.message)),
            Err(Unclosed::Survived) => recorded.push((
                target.info.index,
                "no header control removed the notification".to_owned(),
            )),
            Err(Unclosed::Bystander) => {
                recorded.push((target.info.index, bystander_error().message));
            }
        }
    }
    let current = read_listing(world, deadline)
        .map_err(unconfirmed_read)?
        .seen();
    let mut failures: Vec<String> = recorded
        .iter()
        .map(|(index, message)| format!("#{index}: {message}"))
        .collect();
    failures.extend(
        remaining(&captured, &current)
            .iter()
            .filter(|seen| !recorded.iter().any(|(index, _)| *index == seen.info.index))
            .map(|seen| survivor_line(seen.info.index)),
    );
    let dismissed = departed(&captured, &current)
        .into_iter()
        .map(|seen| seen.info.clone())
        .collect();
    Ok((dismissed, failures))
}

#[cfg(test)]
#[path = "dismiss_all_tests.rs"]
mod tests;
