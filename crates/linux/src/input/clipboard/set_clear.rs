#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::{
    AdapterError, ClipboardContent, ClipboardFormat, Deadline, ErrorCode, InteractionLease,
};

fn not_built() -> AdapterError {
    AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "The Linux clipboard is not built yet",
    )
}

pub(crate) fn get_clipboard(
    _format: ClipboardFormat,
    _deadline: Deadline,
) -> Result<Option<ClipboardContent>, AdapterError> {
    Err(not_built())
}

pub(crate) fn set_clipboard(
    _content: &ClipboardContent,
    _lease: &InteractionLease,
) -> Result<(), AdapterError> {
    Err(not_built())
}

pub(crate) fn clear_clipboard(_lease: &InteractionLease) -> Result<(), AdapterError> {
    Err(not_built())
}

pub(crate) fn set_x11(
    _display: &str,
    _content: &ClipboardContent,
    _lease: &InteractionLease,
) -> Result<(), AdapterError> {
    Err(not_built())
}

pub(crate) fn clear_x11(_display: &str, _lease: &InteractionLease) -> Result<(), AdapterError> {
    Err(not_built())
}
