use agent_desktop_core::{AdapterError, InteractionLease, InteractionPolicy, MouseButton, Point};

use crate::system::SharedBus;
use crate::tree::AtspiElement;

pub(crate) fn click(
    _bus: &SharedBus,
    _node: &AtspiElement,
    _button: MouseButton,
    _policy: InteractionPolicy,
    _lease: &InteractionLease,
) -> Result<Point, AdapterError> {
    Err(AdapterError::not_supported("shell pointer click"))
}

#[cfg(test)]
#[path = "shell_click_tests.rs"]
mod tests;
