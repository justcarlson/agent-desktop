#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::AdapterError;

pub(crate) fn unsupported() -> AdapterError {
    AdapterError::not_supported("dismiss_notification")
}

#[cfg(test)]
#[path = "dismiss_tests.rs"]
mod tests;
