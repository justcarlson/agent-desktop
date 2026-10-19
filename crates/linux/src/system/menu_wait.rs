use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, ProcessIdentity};

use super::SharedBus;
use super::liveness::Liveness;

#[expect(dead_code, reason = "stub the owning lane replaces")]
pub(super) trait MenuWorld {
    fn liveness(&self) -> Liveness;
    fn menu_open(&self, deadline: Deadline) -> Result<bool, AdapterError>;
    fn pause(&self, gap: Duration);
}

pub(crate) fn wait_for_menu(
    _bus: &SharedBus,
    _process: ProcessIdentity,
    _open: bool,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    Err(deadline.timeout_error())
}

#[cfg(test)]
#[path = "menu_wait_tests.rs"]
mod tests;
