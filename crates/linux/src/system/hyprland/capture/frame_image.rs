//! A captured frame turned into the PNG the screenshot command returns.

use agent_desktop_core::{AdapterError, ErrorCode, ImageBuffer};

use crate::system::capture::pixels::to_rgb;
use crate::system::capture::png_encode::encode_png;
use crate::system::wayland::capture::raw_frame::RawFrame;
use crate::system::wayland::capture::shm_layout::{upright, visual_of};

/// The PNG of `frame` with its rows upright, reported at `scale` buffer
/// pixels per logical pixel.
pub(crate) fn frame_image(frame: &RawFrame, scale: f64) -> Result<ImageBuffer, AdapterError> {
    let visual = visual_of(frame.format).ok_or_else(|| {
        AdapterError::new(
            ErrorCode::ActionFailed,
            format!(
                "The frame uses wl_shm format {:#x}, which this tool does not read",
                frame.format
            ),
        )
    })?;
    let rgb = to_rgb(
        &visual,
        &frame.data,
        frame.width,
        frame.height,
        frame.stride as usize,
    )?;
    let rgb = upright(rgb, frame.width, frame.height, frame.inverted);
    encode_png(&rgb, frame.width, frame.height, scale)
}

/// Buffer pixels per logical pixel for a frame `pixels` wide of something
/// `logical` wide, one when the logical width is unknown.
pub(crate) fn scale_of(pixels: u32, logical: f64) -> f64 {
    if logical.is_finite() && logical > 0.0 {
        f64::from(pixels) / logical
    } else {
        1.0
    }
}

#[cfg(test)]
#[path = "frame_image_tests.rs"]
mod tests;
