#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

use agent_desktop_core::{Action, ActionRequest, ActionResult, AdapterError, InteractionLease};

use crate::system::SharedBus;
use crate::tree::AtspiElement;

pub(crate) fn click(
    _bus: &SharedBus,
    _action: &Action,
    _element: Option<&AtspiElement>,
    _request: &ActionRequest,
    _lease: &InteractionLease,
) -> Result<ActionResult, AdapterError> {
    Err(AdapterError::not_supported("click"))
}

#[cfg(test)]
#[path = "physical_click_tests.rs"]
mod tests;
