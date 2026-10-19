use agent_desktop_core::{AdapterError, Deadline, ErrorCode, ImageBuffer, Rect, WindowInfo};

use super::display::scale_for;
use super::pixels::MAX_PIXELS;
use super::window_context::{CaptureEnv, WindowContext, grab_failure};
use super::window_obstruction::{Cover, cover};
use super::window_unit::{Unit, choose, for_screenshot};
use super::window_xid::xid_or_error;
use crate::system::bus::ensure_budget;
use crate::system::x11::grab_rect::GrabRect;
use crate::system::x11::map_state::MapState;
use crate::tree::Extents;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Geometry {
    client: Extents,
    frame: Option<Extents>,
    frame_id: Option<u32>,
}

fn refusal(code: ErrorCode, message: &str, suggestion: &str) -> AdapterError {
    AdapterError::new(code, message).with_suggestion(suggestion)
}

fn covered(known: bool) -> AdapterError {
    let (message, reason) = if known {
        ("Another window is drawn over this window", "covered")
    } else {
        (
            "Whether another window is drawn over this window could not be read",
            "covered_unknown",
        )
    };
    refusal(
        ErrorCode::ActionFailed,
        message,
        "Raise the window or move what covers it, then retry",
    )
    .with_details(serde_json::json!({ "reason": reason }))
}

fn survey(ctx: &WindowContext) -> Result<Geometry, AdapterError> {
    let grabber = ctx.env.grabber;
    let client = grabber
        .describe(ctx.xid, ctx.deadline)
        .map_err(|error| grab_failure(ctx.env, error, ctx.deadline))?;
    if !matches!(client.map_state, MapState::Viewable) {
        return Err(refusal(
            ErrorCode::InvalidArgs,
            "The window is minimised or not shown, so it has no pixels to capture",
            "Restore the window, then retry",
        ));
    }
    let frame = match client.parent {
        Some(parent) => {
            let described = grabber
                .describe(parent, ctx.deadline)
                .map_err(|error| grab_failure(ctx.env, error, ctx.deadline))?;
            Some((parent, described.rect))
        }
        None => None,
    };
    Ok(Geometry {
        client: client.rect,
        frame: frame.map(|(_, rect)| rect),
        frame_id: frame.map(|(id, _)| id),
    })
}

fn unit_of(geometry: &Geometry, bounds: Option<Rect>, strict: bool) -> Result<Unit, AdapterError> {
    let proven = choose(geometry.client, geometry.frame, bounds);
    if proven == Unit::Unproven && strict {
        return Err(refusal(
            ErrorCode::ActionFailed,
            "No drawable of the window matches the bounds the window reports",
            "Capture the window with 'screenshot --window-id' instead",
        ));
    }
    Ok(for_screenshot(proven, geometry.frame.is_some()))
}

fn target_of(geometry: &Geometry, unit: Unit, xid: u32) -> (u32, Extents) {
    match (unit, geometry.frame_id, geometry.frame) {
        (Unit::Frame, Some(id), Some(rect)) => (id, rect),
        _ => (xid, geometry.client),
    }
}

fn grab_rect_of(rect: Extents) -> Result<GrabRect, AdapterError> {
    let too_large = || {
        refusal(
            ErrorCode::ActionFailed,
            "The window is too large to capture",
            "Make the window smaller, then retry",
        )
    };
    let width = u16::try_from(rect.width).map_err(|_| too_large())?;
    let height = u16::try_from(rect.height).map_err(|_| too_large())?;
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(too_large());
    }
    Ok(GrabRect {
        x: 0,
        y: 0,
        width,
        height,
    })
}

fn require_clear(ctx: &WindowContext, rect: Extents) -> Result<(), AdapterError> {
    if ctx.compositing {
        return Ok(());
    }
    let read = ctx.env.stacking.read_within(ctx.xid, ctx.deadline);
    match cover(&read, ctx.xid, rect) {
        Cover::Clear => Ok(()),
        Cover::Covered => Err(covered(true)),
        Cover::Unknown => Err(covered(false)),
    }
}

fn changed() -> AdapterError {
    AdapterError::stale_ref_because("The window changed while it was being captured")
        .with_suggestion("Run 'list-windows' to refresh window identifiers, then retry")
}

fn recheck(ctx: &WindowContext, before: &Geometry, rect: Extents) -> Result<(), AdapterError> {
    ensure_budget(ctx.deadline)?;
    ctx.env.lookup.resolve(&ctx.window, ctx.deadline)?;
    let xid = xid_or_error(ctx.env.lookup.xid(&ctx.window, ctx.deadline))?;
    if xid != ctx.xid {
        return Err(changed());
    }
    match survey(ctx) {
        Ok(after) if after == *before => {}
        Ok(_) => return Err(changed()),
        Err(error) if error.code == ErrorCode::InvalidArgs => return Err(changed()),
        Err(error) => return Err(error),
    }
    require_clear(ctx, rect)
}

/// Captures one window's own drawable. The window is resolved strictly and
/// mapped to its X11 id before the grab and both are proven again after it, so
/// the pixels belong to the window that was asked for. `strict_unit` refuses a
/// window whose drawables match no reported bounds.
pub(crate) fn capture(
    env: &CaptureEnv,
    window: &WindowInfo,
    strict_unit: bool,
    deadline: Deadline,
) -> Result<ImageBuffer, AdapterError> {
    if window.process_instance.is_none() {
        return Err(refusal(
            ErrorCode::InvalidArgs,
            "Exact window screenshot requires a process instance token",
            "Refresh the target with 'list-windows', then retry",
        ));
    }
    ensure_budget(deadline)?;
    let resolved = env.lookup.resolve(window, deadline)?;
    let xid = xid_or_error(env.lookup.xid(&resolved, deadline))?;
    ensure_budget(deadline)?;
    let compositing = env
        .grabber
        .compositing(deadline)
        .map_err(|error| grab_failure(env, error, deadline))?;
    let ctx = WindowContext {
        env,
        window: resolved,
        xid,
        compositing,
        deadline,
    };
    let before = survey(&ctx)?;
    let unit = unit_of(&before, ctx.window.bounds, strict_unit)?;
    let (drawable, rect) = target_of(&before, unit, xid);
    let area = grab_rect_of(rect)?;
    require_clear(&ctx, rect)?;
    ensure_budget(deadline)?;
    let raw = env
        .grabber
        .grab(drawable, area, deadline)
        .map_err(|error| grab_failure(env, error, deadline))?;
    recheck(&ctx, &before, rect)?;
    let displays = env.lookup.displays(deadline).unwrap_or_default();
    let scale = scale_for(&displays, ctx.window.bounds.unwrap_or_else(|| rect.into()));
    let rgb = (env.decode)(&raw)?;
    (env.encode)(&rgb, raw.width, raw.height, scale)
}

#[cfg(test)]
#[path = "window_fakes_tests.rs"]
pub(crate) mod fakes;

#[cfg(test)]
#[path = "window_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "window_guard_tests.rs"]
mod guard_tests;
