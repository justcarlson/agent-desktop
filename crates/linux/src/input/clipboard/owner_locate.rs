#![expect(dead_code, reason = "stub the owning lane replaces")]

use std::path::PathBuf;

use agent_desktop_core::{AdapterError, ErrorCode};

pub(crate) fn locate_helper() -> Result<PathBuf, AdapterError> {
    Err(AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "The clipboard owner helper is not built yet",
    ))
}
