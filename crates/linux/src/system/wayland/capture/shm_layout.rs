//! How the pixels of a shared-memory capture buffer are laid out: which
//! `wl_shm` formats this tool reads, which one it asks for, and rows put
//! back upright when a compositor reports them inverted. Every format here is a little-endian 32-bit word with
//! one byte per channel, so the X11 pixel reader decodes it unchanged.

use crate::system::capture::pixels::VisualFormat;

pub(crate) const ARGB8888: u32 = 0;
pub(crate) const XRGB8888: u32 = 1;
const XBGR8888: u32 = 0x3432_4258;
const ABGR8888: u32 = 0x3432_4241;
const PREFERRED: [u32; 4] = [XRGB8888, XBGR8888, ARGB8888, ABGR8888];

/// The reader's view of a `wl_shm` format this tool can decode.
pub(crate) fn visual_of(format: u32) -> Option<VisualFormat> {
    let (red_mask, blue_mask) = match format {
        ARGB8888 | XRGB8888 => (0x00ff_0000, 0x0000_00ff),
        XBGR8888 | ABGR8888 => (0x0000_00ff, 0x00ff_0000),
        _ => return None,
    };
    Some(VisualFormat {
        red_mask,
        green_mask: 0x0000_ff00,
        blue_mask,
        bits_per_pixel: 32,
        depth: 24,
        scanline_pad: 32,
        msb_first: false,
    })
}

/// The format to ask for among those the compositor offered: an opaque one
/// first, so no alpha byte is read as colour.
pub(crate) fn choose_format(offered: &[u32]) -> Option<u32> {
    PREFERRED
        .into_iter()
        .find(|format| offered.contains(format))
}

/// The pixels with their rows in top-to-bottom order.
pub(crate) fn upright(rgb: Vec<u8>, width: u32, height: u32, inverted: bool) -> Vec<u8> {
    if !inverted {
        return rgb;
    }
    let row = width as usize * 3;
    if row == 0 {
        return rgb;
    }
    rgb.chunks(row)
        .take(height as usize)
        .rev()
        .flatten()
        .copied()
        .collect()
}

#[cfg(test)]
#[path = "shm_layout_tests.rs"]
mod tests;
