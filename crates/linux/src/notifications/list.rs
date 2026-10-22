use std::time::Duration;

use agent_desktop_core::{
    AdapterError, Deadline, ErrorCode, InteractionLease, InteractionPolicy, NotificationFilter,
    NotificationInfo,
};

use super::extract::{Entry, extract, partition};
use super::seen::Seen;
use super::world::{LiveWorld, ShellWorld};
use crate::system::SharedBus;
use crate::system::shell::container::Landmark;

const UNREADABLE_ENTRY: &str = "A notification in the message list has no readable title";
const PARTIAL_READ: &str = "The message list could only be read in part";
const LABEL_RETRIES: usize = 12;
const LABEL_RETRY_GAP: Duration = Duration::from_millis(100);

/// What one read of the message list found: its entries in tree order, with
/// indices over the unfiltered list.
pub(crate) struct Listing {
    pub(crate) entries: Vec<Entry>,
}

impl Listing {
    pub(crate) fn infos(&self) -> Vec<NotificationInfo> {
        self.entries
            .iter()
            .map(|entry| entry.info.clone())
            .collect()
    }

    pub(crate) fn seen(&self) -> Vec<Seen> {
        self.entries
            .iter()
            .map(|entry| Seen {
                info: entry.info.clone(),
                node: entry.node.clone(),
            })
            .collect()
    }
}

pub(crate) fn list(
    bus: &SharedBus,
    filter: &NotificationFilter,
    _policy: InteractionPolicy,
    deadline: Deadline,
    _lease: Option<&InteractionLease>,
) -> Result<Vec<NotificationInfo>, AdapterError> {
    list_in(&LiveWorld::new(bus), filter, deadline)
}

pub(crate) fn list_in(
    world: &dyn ShellWorld,
    filter: &NotificationFilter,
    deadline: Deadline,
) -> Result<Vec<NotificationInfo>, AdapterError> {
    let listing = read_listing(world, deadline)?;
    Ok(apply_filter(listing.infos(), filter))
}

/// Reads the message list through its container only, so banner copies never
/// contribute. A container the footer row proves is an empty list when it
/// holds no entry; a cut-short read is an error, so empty and unreadable stay
/// apart.
pub(crate) fn read_listing(
    world: &dyn ShellWorld,
    deadline: Deadline,
) -> Result<Listing, AdapterError> {
    world.require_daemon(deadline)?;
    let mut tries = 0;
    loop {
        match read_listing_once(world, deadline) {
            Err(error)
                if matches!(error.message.as_str(), UNREADABLE_ENTRY | PARTIAL_READ)
                    && tries < LABEL_RETRIES =>
            {
                tries += 1;
                if deadline.is_expired() {
                    return Err(error);
                }
                std::thread::sleep(LABEL_RETRY_GAP);
            }
            other => return other,
        }
    }
}

fn read_listing_once(world: &dyn ShellWorld, deadline: Deadline) -> Result<Listing, AdapterError> {
    let container = world.locate(Landmark::MessageList, deadline)?;
    let reading = world.read(&container, deadline)?;
    if !reading.complete {
        return Err(incomplete_read(deadline));
    }
    let mut entries = Vec::new();
    for (position, subtree) in partition(&reading.nodes).into_iter().enumerate() {
        entries.push(extract(subtree, position + 1).ok_or_else(unreadable_entry)?);
    }
    Ok(Listing { entries })
}

fn incomplete_read(deadline: Deadline) -> AdapterError {
    if deadline.is_expired() {
        return deadline.timeout_error();
    }
    AdapterError::new(ErrorCode::Internal, PARTIAL_READ)
        .with_suggestion("Retry the command; the shell tree changed while it was read")
}

fn unreadable_entry() -> AdapterError {
    AdapterError::new(ErrorCode::Internal, UNREADABLE_ENTRY)
        .with_suggestion("Retry the command; report the platform detail if it persists")
}

/// Filters keep the indices the unfiltered read assigned, then apply the
/// limit, so a filtered entry is still addressed by its place in the list.
pub(crate) fn apply_filter(
    entries: Vec<NotificationInfo>,
    filter: &NotificationFilter,
) -> Vec<NotificationInfo> {
    entries
        .into_iter()
        .filter(|info| matches_filters(info, filter))
        .take(filter.limit.unwrap_or(usize::MAX))
        .collect()
}

pub(crate) fn matches_app(info: &NotificationInfo, app: &str) -> bool {
    info.app_name.to_lowercase().contains(&app.to_lowercase())
}

fn matches_filters(info: &NotificationInfo, filter: &NotificationFilter) -> bool {
    if let Some(app) = filter.app.as_deref()
        && !matches_app(info, app)
    {
        return false;
    }
    if let Some(text) = filter.text.as_deref() {
        let haystack = format!(
            "{} {} {}",
            info.title,
            info.body.as_deref().unwrap_or(""),
            info.app_name
        )
        .to_lowercase();
        if !haystack.contains(&text.to_lowercase()) {
            return false;
        }
    }
    true
}

#[cfg(test)]
#[path = "list_tests.rs"]
mod tests;
