#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

use crate::system::display_server::DisplayServer;

pub(crate) fn eligible(_role: u32, _interfaces: &[String], _session: &DisplayServer) -> bool {
    false
}

#[cfg(test)]
#[path = "scroll_capability_tests.rs"]
mod tests;
