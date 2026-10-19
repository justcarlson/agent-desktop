#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::AdapterError;

pub(crate) fn unresolved() -> AdapterError {
    AdapterError::not_supported("resolving a tray menu entry")
}

#[cfg(test)]
#[path = "menu_resolve_tests.rs"]
mod tests;
