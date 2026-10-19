#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::AdapterError;

pub(crate) fn unsupported() -> AdapterError {
    AdapterError::not_supported("activating a tray item")
}

#[cfg(test)]
#[path = "activate_tests.rs"]
mod tests;
