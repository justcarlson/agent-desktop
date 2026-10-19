#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

use agent_desktop_core::{AdapterError, Deadline, DragParams};

use super::session;
use crate::input::drag_plan::plan_drag;

pub(crate) fn drag(
    display: &str,
    params: &DragParams,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    let plan = plan_drag(params, deadline)?;
    session::run(display, vec![plan], &mut || Ok(()), deadline).map(|_| ())
}

#[cfg(test)]
#[path = "drag_send_tests.rs"]
mod tests;
