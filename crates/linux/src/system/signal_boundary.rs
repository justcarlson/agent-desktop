#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::{AdapterError, Deadline};

use super::SharedBus;

pub(crate) enum Scope {
    Unscoped,
    Scoped,
}

pub(crate) fn narrow(
    error: AdapterError,
    _scope: Scope,
    _bus: &SharedBus,
    _deadline: Deadline,
) -> AdapterError {
    error
}

#[cfg(test)]
#[path = "signal_boundary_tests.rs"]
mod tests;
