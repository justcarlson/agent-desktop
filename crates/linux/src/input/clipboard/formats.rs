#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::{AdapterError, ErrorCode};

pub(crate) fn decode_text(_target: &str, _bytes: &[u8]) -> Result<String, AdapterError> {
    Err(AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "Text decoding is not built yet",
    ))
}
