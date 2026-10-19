#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::{AdapterError, ErrorCode, ImageBuffer};

pub(crate) fn encode_png(
    _rgb: &[u8],
    _width: u32,
    _height: u32,
    _scale_factor: f64,
) -> Result<ImageBuffer, AdapterError> {
    Err(AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "PNG encoding is not built yet",
    ))
}
