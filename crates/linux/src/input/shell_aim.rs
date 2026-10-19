#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::{AdapterError, DisplayInfo, Point, Rect};

pub(crate) fn aim(_extents: Rect, _displays: &[DisplayInfo]) -> Result<Point, AdapterError> {
    Err(AdapterError::not_supported("shell pointer aim"))
}

#[cfg(test)]
#[path = "shell_aim_tests.rs"]
mod tests;
