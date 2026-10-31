//! Hyprland `screenshot`. A window is captured through
//! `ext_image_copy_capture_v1` from its own foreign toplevel, so a window
//! on a hidden or special workspace is captured without being shown or
//! activated. A display is captured through `zwlr_screencopy_v1` from the
//! output Hyprland names it by. The permission setting is read before
//! anything is sent, and no portal is ever asked.

use agent_desktop_core::{AdapterError, ErrorCode, ImageBuffer, ScreenshotTarget, WindowInfo};

use super::capture_error::permission_refusal;
use super::capture_world::CaptureWorld;
use super::cursor_hider::CursorHider;
use super::frame_image::{frame_image, scale_of};
use crate::system::capture::display::select;
use crate::system::wayland::capture::raw_frame::RawFrame;

const SIZE_TOLERANCE: f64 = 1.0;

/// Captures exactly the toplevel `win` is bound to.
pub(crate) fn capture_window(
    world: &mut impl CaptureWorld,
    win: &WindowInfo,
) -> Result<ImageBuffer, AdapterError> {
    refuse_enforced(world)?;
    let bound = world.bind(win)?;
    let frame = world.capture_toplevel(&bound.toplevel.identifier)?;
    if !world.still_bound(&bound)? {
        return Err(AdapterError::new(
            ErrorCode::WindowNotFound,
            "The window closed or changed while its frame was captured",
        )
        .with_suggestion("Run 'list-windows' for the current window ids, then retry"));
    }
    frame_image(&frame, scale_of(frame.width, bound.client.geometry.width))
}

/// Captures the display `target` names, with `hider` hiding what the agent
/// draws over it for the length of the capture.
pub(crate) fn capture_screen(
    world: &mut impl CaptureWorld,
    hider: &mut impl CursorHider,
    target: &ScreenshotTarget,
) -> Result<ImageBuffer, AdapterError> {
    refuse_enforced(world)?;
    let displays = world.displays()?;
    let (_, display) = select(&displays, target)?;
    hider.hide()?;
    let frame = world.capture_output(&display.id);
    hider.show()?;
    let frame = frame?;
    if !world.displays()?.contains(&display) {
        return Err(AdapterError::new(
            ErrorCode::InvalidArgs,
            format!(
                "Display {} changed while its frame was captured",
                display.id
            ),
        )
        .with_suggestion("Run 'list-displays' to see the displays that exist now, then retry"));
    }
    check_size(
        &frame,
        display.bounds.width,
        display.bounds.height,
        display.scale,
    )?;
    frame_image(&frame, display.scale)
}

fn refuse_enforced(world: &mut impl CaptureWorld) -> Result<(), AdapterError> {
    if world.enforced()? {
        Err(permission_refusal())
    } else {
        Ok(())
    }
}

fn check_size(frame: &RawFrame, width: f64, height: f64, scale: f64) -> Result<(), AdapterError> {
    let fits =
        |pixels: u32, logical: f64| (f64::from(pixels) - logical * scale).abs() <= SIZE_TOLERANCE;
    if fits(frame.width, width) && fits(frame.height, height) {
        return Ok(());
    }
    Err(AdapterError::new(
        ErrorCode::ActionFailed,
        format!(
            "The frame is {}x{} pixels, but the display is {width}x{height} at scale {scale}",
            frame.width, frame.height
        ),
    )
    .with_suggestion("Retry once the display configuration settles"))
}

#[cfg(test)]
#[path = "hypr_capture_tests.rs"]
mod tests;
