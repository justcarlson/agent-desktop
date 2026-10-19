use agent_desktop_core::{AdapterError, Deadline};

use crate::system::SharedBus;

pub(crate) fn verify(
    _bus: &SharedBus,
    _window_id: &str,
    _deadline: Deadline,
) -> Result<(), AdapterError> {
    Err(AdapterError::not_supported("shell window verification"))
}

#[cfg(test)]
#[path = "window_gate_tests.rs"]
mod tests;
