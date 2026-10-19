#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::{AdapterError, ErrorCode};

pub(crate) fn parse_uri_list(_bytes: &[u8]) -> Result<Vec<String>, AdapterError> {
    Err(AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "URI list parsing is not built yet",
    ))
}
