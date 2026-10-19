#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

use agent_desktop_core::{AdapterError, Deadline};

use crate::input::step::{Delivery, KeysymStep};

pub(crate) fn send_keys(
    _keys: &[KeysymStep],
    _gate: &mut dyn FnMut() -> Result<(), AdapterError>,
    _deadline: Deadline,
) -> Result<Delivery, AdapterError> {
    Err(AdapterError::not_supported("wayland_keys"))
}

pub(crate) fn pointer_refusal(what: &'static str) -> AdapterError {
    AdapterError::not_supported(what)
}

#[cfg(test)]
#[path = "keysym_send_tests.rs"]
mod tests;
