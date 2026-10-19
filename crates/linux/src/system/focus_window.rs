use agent_desktop_core::{AdapterError, InteractionLease, WindowInfo};

pub(crate) fn focus_window(
    _win: &WindowInfo,
    _lease: &InteractionLease,
) -> Result<(), AdapterError> {
    Err(AdapterError::not_supported("focus_window"))
}
