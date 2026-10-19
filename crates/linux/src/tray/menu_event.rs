#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::AdapterError;

pub(crate) fn unsupported() -> AdapterError {
    AdapterError::not_supported("picking a tray menu entry")
}

#[cfg(test)]
#[path = "menu_event_tests.rs"]
mod tests;
