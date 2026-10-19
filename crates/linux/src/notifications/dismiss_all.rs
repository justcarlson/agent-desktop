use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, DismissAllNotificationsRequest, ErrorCode,
    InteractionLease, MouseButton, NotificationFilter, NotificationInfo,
};

use super::extract::Control;
use super::list::{Listing, apply_filter, read_listing};
use super::session::Session;
use super::verify::{
    Removal, bystander_error, judge_removal, lost, same_identity, settle_read, survivor_line,
    survivors,
};
use super::world::{LiveWorld, ShellWorld};
use crate::system::SharedBus;

type Outcome = (Vec<NotificationInfo>, Vec<String>);
type Closed = (Vec<NotificationInfo>, Vec<(usize, String)>);

pub(crate) fn dismiss_all(
    bus: &SharedBus,
    request: DismissAllNotificationsRequest<'_>,
    lease: &InteractionLease,
) -> Result<Outcome, AdapterError> {
    dismiss_all_in(&LiveWorld::new(bus), request, lease)
}

/// Clears the list, or only the entries of one app. Without a filter it
/// clicks the clear-all button once; with a filter it never does, because
/// that button has no notion of an app, and closes the matching entries one
/// by one by identity. The outcome is judged against the set captured first.
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

fn run(
    world: &dyn ShellWorld,
    request: &DismissAllNotificationsRequest<'_>,
    lease: &InteractionLease,
) -> Result<Outcome, AdapterError> {
    let deadline = lease.deadline();
    let listing = read_listing(world, deadline)?;
    let filter = app_scope(request);
    let captured = apply_filter(listing.infos(), &filter);
    if captured.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let (current, recorded) = if request.app_filter.is_some() {
        close_each(world, &captured, &filter, request, lease)?
    } else {
        (
            click_clear_all(world, listing, &captured, request, lease)?,
            Vec::new(),
        )
    };
    let mut failures: Vec<String> = recorded
        .iter()
        .map(|(index, message)| format!("#{index}: {message}"))
        .collect();
    failures.extend(
        survivors(&captured, &current)
            .iter()
            .filter(|member| !recorded.iter().any(|(index, _)| *index == member.index))
            .map(|member| survivor_line(member.index)),
    );
    let dismissed = lost(&captured, &current);
    Ok((dismissed, failures))
}

fn app_scope(request: &DismissAllNotificationsRequest<'_>) -> NotificationFilter {
    NotificationFilter {
        app: request.app_filter.map(str::to_owned),
        ..NotificationFilter::default()
    }
}

fn click_clear_all(
    world: &dyn ShellWorld,
    listing: Listing,
    captured: &[NotificationInfo],
    request: &DismissAllNotificationsRequest<'_>,
    lease: &InteractionLease,
) -> Result<Vec<NotificationInfo>, AdapterError> {
    let clear: Control = listing.clear_all.ok_or_else(missing_clear_all)?;
    world.click(&clear.element, MouseButton::Left, request.policy, lease)?;
    settle_read(world, lease.deadline(), |now| {
        survivors(captured, now).is_empty()
    })
}

fn close_each(
    world: &dyn ShellWorld,
    captured: &[NotificationInfo],
    filter: &NotificationFilter,
    request: &DismissAllNotificationsRequest<'_>,
    lease: &InteractionLease,
) -> Result<Closed, AdapterError> {
    let deadline = lease.deadline();
    let mut current = Vec::new();
    let mut recorded = Vec::new();
    for target in captured {
        let listing = read_listing(world, deadline)?;
        let before = listing.infos();
        let live = listing.entries.into_iter().find(|entry| {
            same_identity(&entry.info, target)
                && apply_filter(vec![entry.info.clone()], filter).len() == 1
        });
        let Some(entry) = live else {
            current = before;
            continue;
        };
        let Some(close) = entry.close else {
            recorded.push((target.index, "the entry shows no close button".to_owned()));
            current = before;
            continue;
        };
        if let Err(error) = world.click(&close.element, MouseButton::Left, request.policy, lease) {
            recorded.push((target.index, error.message));
            current = before;
            continue;
        }
        current = settle_read(world, deadline, |now| {
            judge_removal(&before, now, target) != Removal::Pending
        })?;
        if judge_removal(&before, &current, target) == Removal::Bystander {
            recorded.push((target.index, bystander_error().message));
        }
    }
    Ok((current, recorded))
}

fn missing_clear_all() -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "The message list shows no clear-all button to click",
    )
    .with_suggestion("Run list-notifications and retry with a freshly observed list")
    .with_disposition(DeliverySemantics::not_delivered())
}

#[cfg(test)]
#[path = "dismiss_all_tests.rs"]
mod tests;
