#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::{AdapterError, Deadline};

pub(crate) fn registered_items(_deadline: Deadline) -> Result<Vec<String>, AdapterError> {
    Err(AdapterError::not_supported("reading the tray watcher"))
}

#[cfg(test)]
#[path = "watcher_tests.rs"]
mod tests;
