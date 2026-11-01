//! The extent of an arrow drawn for one output scale: its size in physical
//! pixels and where its tip sits inside it.

use super::arrow_bitmap::ArrowBitmap;

/// An arrow's physical size and hotspot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ArrowFrame {
    pub(crate) size: (u32, u32),
    pub(crate) hotspot: (f64, f64),
}

impl ArrowFrame {
    /// The frame `bitmap` fills.
    pub(crate) fn of(bitmap: &ArrowBitmap) -> Self {
        Self {
            size: (bitmap.width, bitmap.height),
            hotspot: bitmap.hotspot,
        }
    }
}
