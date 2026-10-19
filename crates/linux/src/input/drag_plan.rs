#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

use agent_desktop_core::{AdapterError, Deadline, DragParams};

use super::step::Plan;

pub(crate) fn plan_drag(_params: &DragParams, _deadline: Deadline) -> Result<Plan, AdapterError> {
    Err(AdapterError::not_supported("plan_drag"))
}

#[cfg(test)]
#[path = "drag_plan_tests.rs"]
mod tests;
