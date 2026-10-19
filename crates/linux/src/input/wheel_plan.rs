#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

use agent_desktop_core::{AdapterError, Modifier, Point};

use super::keymap::Keymap;
use super::step::Plan;

pub(crate) fn plan_wheel(
    _map: &Keymap,
    _at: Point,
    _dx: f64,
    _dy: f64,
    _mods: &[Modifier],
) -> Result<Vec<Plan>, AdapterError> {
    Err(AdapterError::not_supported("plan_wheel"))
}

#[cfg(test)]
#[path = "wheel_plan_tests.rs"]
mod tests;
