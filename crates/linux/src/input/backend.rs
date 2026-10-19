#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

use agent_desktop_core::AdapterError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Backend {
    X11(String),
    Wayland,
}

pub(crate) fn backend() -> Result<Backend, AdapterError> {
    Err(AdapterError::not_supported("input_backend"))
}

#[cfg(test)]
#[path = "backend_tests.rs"]
mod tests;
