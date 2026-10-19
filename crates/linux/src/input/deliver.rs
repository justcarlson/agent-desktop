#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

use agent_desktop_core::{AdapterError, Deadline};

use super::step::{Delivery, KeysymStep, Plan};

pub(crate) fn send_plans(
    _plans: Vec<Plan>,
    _gate: &mut dyn FnMut() -> Result<(), AdapterError>,
    _deadline: Deadline,
) -> Result<Delivery, AdapterError> {
    Err(AdapterError::not_supported("send_plans"))
}

pub(crate) fn send_keysyms(
    _keys: &[KeysymStep],
    _gate: &mut dyn FnMut() -> Result<(), AdapterError>,
    _deadline: Deadline,
) -> Result<Delivery, AdapterError> {
    Err(AdapterError::not_supported("send_keysyms"))
}

#[cfg(test)]
#[path = "deliver_tests.rs"]
mod tests;
