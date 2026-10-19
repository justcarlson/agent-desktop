#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::{AdapterError, Deadline};

use super::identity::ShellRoot;
use crate::system::SharedBus;
use crate::tree::AtspiElement;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Landmark {
    MessageList,
    TrayPanel,
    DockPanel,
    SystemMenu,
    DateMenu,
}

pub(crate) fn locate(
    _bus: &SharedBus,
    _root: &ShellRoot,
    _landmark: Landmark,
    _deadline: Deadline,
) -> Result<AtspiElement, AdapterError> {
    Err(AdapterError::not_supported("shell container lookup"))
}

#[cfg(test)]
#[path = "container_tests.rs"]
mod tests;
