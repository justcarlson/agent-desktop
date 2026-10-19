use agent_desktop_core::{AdapterError, InteractionLease, NativeHandle};

use crate::system::SharedBus;

pub(crate) fn scroll_into_view(
    _bus: &SharedBus,
    _handle: &NativeHandle,
    _lease: &InteractionLease,
) -> Result<(), AdapterError> {
    Err(AdapterError::not_supported("scroll_into_view"))
}
