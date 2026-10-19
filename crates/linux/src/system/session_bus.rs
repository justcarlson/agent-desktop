#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

use agent_desktop_core::{AdapterError, Deadline};

pub(crate) struct SessionBus;

impl SessionBus {
    pub(crate) fn get(_deadline: Deadline) -> Result<zbus::Connection, AdapterError> {
        Err(AdapterError::not_supported("session_bus"))
    }
}

#[cfg(test)]
#[path = "session_bus_tests.rs"]
mod tests;
