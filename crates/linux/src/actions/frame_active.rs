#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

use agent_desktop_core::{AdapterError, Deadline};

use crate::system::SharedBus;
use crate::tree::AtspiElement;

pub(crate) fn frame_active(
    _bus: &SharedBus,
    _frame: &AtspiElement,
    _deadline: Deadline,
) -> Result<Option<bool>, AdapterError> {
    Ok(None)
}

#[cfg(test)]
#[path = "frame_active_tests.rs"]
mod tests;
