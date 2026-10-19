use agent_desktop_core::{ActionResult, AdapterError, InteractionLease, NativeHandle};

use crate::system::SharedBus;

pub(crate) fn scroll_to(
    _bus: &SharedBus,
    _handle: &NativeHandle,
    _lease: &InteractionLease,
) -> Result<ActionResult, AdapterError> {
    Err(AdapterError::not_supported("scroll-to"))
}

#[cfg(test)]
#[path = "scroll_wrap_tests.rs"]
mod tests;
