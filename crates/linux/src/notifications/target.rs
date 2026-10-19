use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, ErrorCode, NotificationIdentity, NotificationInfo,
};

use super::extract::{Control, Entry};
use super::list::{matches_app, read_listing};
use super::verify::same_identity;
use super::world::ShellWorld;

/// A fresh read with the entry a mutation targets picked out of it.
pub(crate) struct Located {
    pub(crate) entry: Entry,
    pub(crate) all: Vec<NotificationInfo>,
}

/// Reads the list and picks the entry at `index`. The index is a position,
/// never the identity: the app filter and the expected identity are checked
/// against what the read found, and a mismatch means the list moved.
pub(crate) fn locate(
    world: &dyn ShellWorld,
    index: usize,
    app_filter: Option<&str>,
    identity: &NotificationIdentity,
    deadline: Deadline,
) -> Result<Located, AdapterError> {
    let listing = read_listing(world, deadline)?;
    let all = listing.infos();
    let entry = listing
        .entries
        .into_iter()
        .find(|entry| entry.info.index == index)
        .ok_or_else(|| AdapterError::notification_not_found(index))?;
    let app_ok = app_filter.is_none_or(|app| matches_app(&entry.info, app));
    if !app_ok || !identity.matches(&entry.info) {
        return Err(moved(index));
    }
    Ok(Located { entry, all })
}

pub(crate) fn moved(index: usize) -> AdapterError {
    AdapterError::new(
        ErrorCode::NotificationNotFound,
        format!("Notification at index {index} no longer matches its expected identity"),
    )
    .with_suggestion("Run list-notifications again and retry with the freshly observed index")
    .with_disposition(DeliverySemantics::not_delivered())
}

/// Reads again right before a click and refuses it when the entry at the
/// index is no longer the one the first read found.
pub(crate) fn reconfirm(
    world: &dyn ShellWorld,
    index: usize,
    app_filter: Option<&str>,
    identity: &NotificationIdentity,
    first: &NotificationInfo,
    deadline: Deadline,
) -> Result<Located, AdapterError> {
    let again = locate(world, index, app_filter, identity, deadline)?;
    if !same_identity(first, &again.entry.info) {
        return Err(moved(index));
    }
    Ok(again)
}

pub(crate) fn missing_control(what: &str, index: usize) -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        format!("The notification at index {index} shows no {what} to click"),
    )
    .with_suggestion("Run list-notifications and retry with a freshly observed entry")
    .with_disposition(DeliverySemantics::not_delivered())
}

pub(crate) fn require(
    control: Option<Control>,
    what: &str,
    index: usize,
) -> Result<Control, AdapterError> {
    control.ok_or_else(|| missing_control(what, index))
}

#[cfg(test)]
#[path = "target_tests.rs"]
mod tests;
