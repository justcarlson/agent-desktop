#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::AdapterError;

pub(crate) fn unsupported() -> AdapterError {
    AdapterError::not_supported("notification_action")
}

#[cfg(test)]
#[path = "action_tests.rs"]
mod tests;
