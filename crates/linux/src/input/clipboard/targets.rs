#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::{AdapterError, ClipboardContent, ClipboardFormat, ErrorCode};

pub(crate) enum Wanted {
    FileUrls(&'static str),
    Image,
    Text(&'static str),
}

pub(crate) fn choose_target(_offered: &[String], _format: ClipboardFormat) -> Option<Wanted> {
    None
}

pub(crate) fn content_of(
    _wanted: &Wanted,
    _bytes: Vec<u8>,
) -> Result<ClipboardContent, AdapterError> {
    Err(AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "Clipboard targets are not built yet",
    ))
}
