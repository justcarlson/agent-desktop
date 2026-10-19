#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

use agent_desktop_core::AdapterError;

use super::keymap::Keymap;
use super::step::{KeysymStep, Plan};

pub(crate) fn plan_text(_map: &Keymap, _text: &str) -> Result<Vec<Plan>, AdapterError> {
    Err(AdapterError::not_supported("plan_text"))
}

pub(crate) fn plan_text_keysyms(_text: &str) -> Vec<Vec<KeysymStep>> {
    Vec::new()
}

#[cfg(test)]
#[path = "text_plan_tests.rs"]
mod tests;
