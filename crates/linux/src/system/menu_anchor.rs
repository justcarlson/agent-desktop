#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::{AdapterError, Deadline, ProcessIdentity};

use super::SharedBus;

pub(crate) fn menu_open(
    _bus: &SharedBus,
    _process: &ProcessIdentity,
    _deadline: Deadline,
) -> Result<bool, AdapterError> {
    Ok(false)
}

#[cfg(test)]
#[path = "menu_anchor_tests.rs"]
mod tests;
