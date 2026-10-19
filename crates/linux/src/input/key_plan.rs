#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

use agent_desktop_core::{AdapterError, KeyCombo};

use super::keymap::Keymap;
use super::step::{KeysymStep, Plan};

pub(crate) fn plan_combo(_map: &Keymap, _combo: &KeyCombo) -> Result<Plan, AdapterError> {
    Err(AdapterError::not_supported("plan_combo"))
}

pub(crate) fn plan_keysyms(_combo: &KeyCombo) -> Result<Vec<KeysymStep>, AdapterError> {
    Err(AdapterError::not_supported("plan_keysyms"))
}

#[cfg(test)]
#[path = "key_plan_tests.rs"]
mod tests;
