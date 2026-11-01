//! Hyprland window operations: bind, read, plan, send, prove.

use agent_desktop_core::{AdapterError, OperationReport, WindowInfo, WindowOp};

use super::window_world::WindowWorld;

/// Applies `op` to `win` and reports the read-back that proves it.
pub(crate) fn window_op_reported(
    _world: &mut impl WindowWorld,
    _win: &WindowInfo,
    _op: &WindowOp,
) -> Result<OperationReport, AdapterError> {
    Err(AdapterError::not_supported("window_op_reported"))
}

#[cfg(test)]
#[path = "hypr_window_op_tests.rs"]
mod tests;
