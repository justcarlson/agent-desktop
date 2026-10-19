#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::{AdapterError, ClipboardContent, ClipboardFormat, Deadline, ErrorCode};

pub(crate) fn read_x11(
    _display: &str,
    _format: ClipboardFormat,
    _deadline: Deadline,
) -> Result<Option<ClipboardContent>, AdapterError> {
    Err(AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "The X11 clipboard read is not built yet",
    ))
}
