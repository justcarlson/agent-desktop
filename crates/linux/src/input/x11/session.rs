#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

use agent_desktop_core::{AdapterError, Deadline};

use crate::input::keymap::Keymap;
use crate::input::step::{Delivery, Plan};

pub(crate) fn run(
    _display: &str,
    _plans: Vec<Plan>,
    _gate: &mut dyn FnMut() -> Result<(), AdapterError>,
    _deadline: Deadline,
) -> Result<Delivery, AdapterError> {
    Err(AdapterError::not_supported("x11_input"))
}

pub(crate) fn live_keymap(_display: &str, _deadline: Deadline) -> Result<Keymap, AdapterError> {
    Err(AdapterError::not_supported("x11_keymap"))
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
