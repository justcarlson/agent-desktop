use agent_desktop_core::{
    AdapterError, ClipboardContent, ClipboardFormat, Deadline, ErrorCode, InteractionLease,
};

fn not_built() -> AdapterError {
    AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "The Wayland clipboard is not built yet",
    )
}

pub(crate) fn read_wayland(
    _format: ClipboardFormat,
    _deadline: Deadline,
) -> Result<Option<ClipboardContent>, AdapterError> {
    Err(not_built())
}

pub(crate) fn set_wayland(
    _content: &ClipboardContent,
    _lease: &InteractionLease,
) -> Result<(), AdapterError> {
    Err(not_built())
}

pub(crate) fn clear_wayland(_lease: &InteractionLease) -> Result<(), AdapterError> {
    Err(not_built())
}
