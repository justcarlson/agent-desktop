use agent_desktop_core::{AdapterError, Deadline};

use crate::system::SharedBus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellRoot {
    pub(crate) bus_name: String,
    pub(crate) path: String,
    pub(crate) pid: u32,
}

pub(crate) fn find_root(_bus: &SharedBus, _deadline: Deadline) -> Result<ShellRoot, AdapterError> {
    Err(AdapterError::not_supported("shell root lookup"))
}

#[cfg(test)]
#[path = "identity_tests.rs"]
mod tests;
