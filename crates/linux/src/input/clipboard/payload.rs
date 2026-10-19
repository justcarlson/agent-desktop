#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::{AdapterError, ClipboardContent, ErrorCode};

pub(crate) struct Target {
    pub(crate) name: String,
    pub(crate) bytes: Vec<u8>,
}

pub(crate) struct Payload {
    pub(crate) targets: Vec<Target>,
}

pub(crate) fn payload_of(_content: &ClipboardContent) -> Result<Payload, AdapterError> {
    Err(AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "Clipboard payloads are not built yet",
    ))
}

pub(crate) fn frame(_payload: &Payload) -> Vec<u8> {
    Vec::new()
}

pub(crate) fn unframe(_bytes: &[u8]) -> Result<Payload, AdapterError> {
    Err(AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "Clipboard payloads are not built yet",
    ))
}
