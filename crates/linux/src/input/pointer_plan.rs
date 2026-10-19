#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

use agent_desktop_core::{AdapterError, Modifier, MouseButton, Point};

use super::keymap::Keymap;
use super::step::Plan;

pub(crate) fn plan_click(
    _map: &Keymap,
    _at: Point,
    _button: &MouseButton,
    _count: u32,
    _mods: &[Modifier],
) -> Result<Plan, AdapterError> {
    Err(AdapterError::not_supported("plan_click"))
}

#[cfg(test)]
#[path = "pointer_plan_tests.rs"]
mod tests;
