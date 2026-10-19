#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::{NotificationFilter, NotificationInfo};

pub(crate) fn apply_filter(
    _entries: Vec<NotificationInfo>,
    _filter: &NotificationFilter,
) -> Vec<NotificationInfo> {
    Vec::new()
}

#[cfg(test)]
#[path = "list_tests.rs"]
mod tests;
