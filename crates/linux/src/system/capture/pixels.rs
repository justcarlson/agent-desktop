#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::{AdapterError, ErrorCode};

pub(crate) const MAX_PIXELS: u64 = 64 * 1024 * 1024;

pub(crate) struct VisualFormat {
    pub(crate) red_mask: u32,
    pub(crate) green_mask: u32,
    pub(crate) blue_mask: u32,
    pub(crate) bits_per_pixel: u8,
    pub(crate) depth: u8,
    pub(crate) scanline_pad: u8,
    pub(crate) msb_first: bool,
}

pub(crate) fn to_rgb(
    _format: &VisualFormat,
    _data: &[u8],
    _width: u32,
    _height: u32,
    _stride: usize,
) -> Result<Vec<u8>, AdapterError> {
    Err(AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "Pixel conversion is not built yet",
    ))
}
