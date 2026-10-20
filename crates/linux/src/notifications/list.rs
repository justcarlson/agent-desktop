use std::time::Duration;

use agent_desktop_core::{
    AdapterError, Deadline, ErrorCode, InteractionLease, InteractionPolicy, NotificationFilter,
    NotificationInfo,
};

use super::extract::{Control, Entry, extract, partition};
use super::landmark::{ContainerState, classify, labels_of};
use super::world::{LiveWorld, Node, Role, ShellWorld};
use crate::system::SharedBus;
use crate::system::shell::container::Landmark;

const UNREADABLE_ENTRY: &str = "A notification in the message list has no readable title";
const LABEL_RETRIES: usize = 12;
const LABEL_RETRY_GAP: Duration = Duration::from_millis(100);

/// What one read of the message list found: its entries in tree order, with
/// indices over the unfiltered list, and the clear-all control outside them.
pub(crate) struct Listing {
    pub(crate) entries: Vec<Entry>,
    pub(crate) clear_all: Option<Control>,
}

impl Listing {
    pub(crate) fn infos(&self) -> Vec<NotificationInfo> {
        self.entries
            .iter()
            .map(|entry| entry.info.clone())
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
/// contribute. A cut-short read or an unrecognised container is an error: the
/// three states of entries, empty and unreadable stay apart.
pub(crate) fn read_listing(
    world: &dyn ShellWorld,
    deadline: Deadline,
) -> Result<Listing, AdapterError> {
    world.require_daemon(deadline)?;
    let mut tries = 0;
    loop {
        match read_listing_once(world, deadline) {
            Err(error) if error.message == UNREADABLE_ENTRY && tries < LABEL_RETRIES => {
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
    let (subtrees, outside) = partition(&reading.nodes);
    let labels = labels_of(&reading.nodes);
    let footer = footer_buttons(&outside);
    match classify(&labels, subtrees.len(), footer.len()) {
        ContainerState::Unrecognised => Err(unrecognised_container()),
        ContainerState::Empty => Ok(Listing {
            entries: Vec::new(),
            clear_all: None,
        }),
        ContainerState::Entries => {
            let mut entries = Vec::with_capacity(subtrees.len());
            for (position, subtree) in subtrees.into_iter().enumerate() {
                entries.push(extract(subtree, position + 1).ok_or_else(unreadable_entry)?);
            }
            let clear_all = footer.last().map(|node| Control {
                name: node.name.clone(),
                extents: node.extents,
                element: node.element.clone(),
            });
            Ok(Listing { entries, clear_all })
        }
    }
}

/// The push buttons outside every entry. The shell keeps one there, the
/// clear-all button, and it is the last of them whatever the language.
fn footer_buttons<'a>(outside: &[&'a Node]) -> Vec<&'a Node> {
    outside
        .iter()
        .copied()
        .filter(|node| node.role == Role::PushButton)
        .collect()
}

fn incomplete_read(deadline: Deadline) -> AdapterError {
    if deadline.is_expired() {
        return deadline.timeout_error();
    }
    AdapterError::new(
        ErrorCode::Internal,
        "The message list could only be read in part",
    )
    .with_suggestion("Retry the command; the shell tree changed while it was read")
}

fn unrecognised_container() -> AdapterError {
    AdapterError::new(
        ErrorCode::ElementNotFound,
        "The message list shows neither notifications nor the empty-list landmark",
    )
    .with_suggestion("Check that the shell is running and its message list is readable")
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
    let kept = entries
        .into_iter()
        .filter(|info| matches_filters(info, filter));
    match filter.limit {
        Some(limit) => kept.take(limit).collect(),
        None => kept.collect(),
    }
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
