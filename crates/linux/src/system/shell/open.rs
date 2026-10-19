#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::{AdapterError, Deadline, InteractionLease, InteractionPolicy, WindowInfo};

use super::kinds::ShellKind;
use crate::system::SharedBus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Opened {
    Adopted,
    Opened,
}

pub(crate) fn open(
    _bus: &SharedBus,
    _kind: ShellKind,
    _policy: InteractionPolicy,
    _lease: &InteractionLease,
) -> Result<(WindowInfo, Opened), AdapterError> {
    Err(AdapterError::not_supported("open_system_surface"))
}

pub(crate) fn close(
    _bus: &SharedBus,
    _kind: ShellKind,
    _lease: &InteractionLease,
) -> Result<(), AdapterError> {
    Err(AdapterError::not_supported("closing a shell surface"))
}

pub(crate) fn is_showing(
    _bus: &SharedBus,
    _kind: ShellKind,
    _deadline: Deadline,
) -> Result<bool, AdapterError> {
    Err(AdapterError::not_supported("reading a shell surface"))
}

#[cfg(test)]
#[path = "open_tests.rs"]
mod tests;
