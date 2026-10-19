#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::NotificationInfo;

pub(crate) fn extract(_labels: &[String], _index: usize) -> Option<NotificationInfo> {
    None
}

#[cfg(test)]
#[path = "extract_tests.rs"]
mod tests;
