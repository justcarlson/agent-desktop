//! What a window operation sends, decided from the window's state before
//! anything is sent. An empty plan means the window already is as asked.

use agent_desktop_core::{AdapterError, WindowOp};

use super::client_state::ClientState;
use super::config_dialect::ConfigDialect;
use super::window_step::WindowStep;

/// The steps of `op` on the window in `state`, with `active` the focused
/// window, or the refusal that keeps the request from being sent.
pub(crate) fn plan(
    _op: &WindowOp,
    _state: &ClientState,
    _active: Option<u64>,
    _dialect: ConfigDialect,
) -> Result<Vec<WindowStep>, AdapterError> {
    Err(AdapterError::not_supported("plan"))
}

#[cfg(test)]
#[path = "window_plan_tests.rs"]
mod tests;
