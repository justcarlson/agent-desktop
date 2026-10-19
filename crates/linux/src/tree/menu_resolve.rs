use agent_desktop_core::{AdapterError, Deadline, NativeHandle, RefEntry};

use crate::system::SharedBus;

pub(crate) fn resolve_menu(
    _bus: &SharedBus,
    _entry: &RefEntry,
    _deadline: Deadline,
) -> Result<NativeHandle, AdapterError> {
    Err(AdapterError::not_supported("resolving a menu item"))
}

#[cfg(test)]
#[path = "menu_resolve_tests.rs"]
mod tests;
