#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::NotificationInfo;

pub(crate) fn settled(_captured: &[NotificationInfo], _remaining: &[NotificationInfo]) -> bool {
    false
}

#[cfg(test)]
#[path = "verify_tests.rs"]
mod tests;
