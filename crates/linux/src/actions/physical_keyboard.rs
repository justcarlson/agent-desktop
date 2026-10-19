#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

use agent_desktop_core::{ActionResult, AdapterError, Deadline, InteractionPolicy, KeyCombo};

use crate::system::SharedBus;

pub(crate) fn press_global(
    _bus: &SharedBus,
    _combo: &KeyCombo,
    _policy: InteractionPolicy,
    _deadline: Deadline,
) -> Result<ActionResult, AdapterError> {
    Err(AdapterError::not_supported("press_global"))
}

#[cfg(test)]
#[path = "physical_keyboard_tests.rs"]
mod tests;
