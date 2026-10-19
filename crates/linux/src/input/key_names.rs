#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

use agent_desktop_core::AdapterError;

pub(crate) fn keysym_for_key(_name: &str) -> Result<u32, AdapterError> {
    Err(AdapterError::not_supported("key_names"))
}

#[cfg(test)]
#[path = "key_names_tests.rs"]
mod tests;
