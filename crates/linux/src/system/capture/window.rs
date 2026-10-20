use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, ErrorCode, ImageBuffer, Rect, WindowInfo,
};

use super::display::scale_for;
use super::pixels::MAX_PIXELS;
use super::window_context::{CaptureEnv, WindowContext, grab_failure};
use super::window_obstruction::{Cover, cover};
use super::window_unit::{Unit, choose, for_screenshot};
use super::window_xid::xid_or_error;
use crate::system::bus::ensure_budget;
use crate::system::x11::FrameExtents;
use crate::system::x11::grab_error::GrabError;
use crate::system::x11::grab_rect::GrabRect;
use crate::system::x11::map_state::MapState;
use crate::tree::Extents;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Geometry {
    client: Extents,
    inset: (i16, i16),
    frame: Option<Extents>,
    frame_id: Option<u32>,
}

fn refusal(code: ErrorCode, message: &str, suggestion: &str) -> AdapterError {
    AdapterError::new(code, message)
        .with_suggestion(suggestion)
        .with_disposition(DeliverySemantics::not_delivered())
}

fn not_viewable(compositing: bool) -> AdapterError {
    let message = if compositing {
        "The window is not viewable, so it has no pixels to capture"
    } else {
        "The window is not viewable and the window manager does not composite, so it has no pixels to capture"
    };
    refusal(
        ErrorCode::InvalidArgs,
        message,
        "Restore the window with 'restore', then retry",
    )
}

fn stopped(window: &WindowInfo) -> AdapterError {
    AdapterError::app_unresponsive(&window.app).with_details(serde_json::json!({
        "pid": window.pid,
        "process_instance": window.process_instance,
    }))
}

fn covered() -> AdapterError {
    refusal(
        ErrorCode::ActionFailed,
        "Another window is drawn over this window and its own pixels could not be read",
        "Raise the window with 'focus-window', or move what covers it, then retry",
    )
    .with_details(serde_json::json!({ "reason": "covered" }))
}

fn inside_shadow(rect: Extents, shadow: Option<FrameExtents>) -> (Extents, (i16, i16)) {
    let Some(shadow) = shadow else {
        return (rect, (0, 0));
    };
    let grown = |value: u32| i32::try_from(value).ok();
    let (Some(left), Some(right), Some(top), Some(bottom)) = (
        grown(shadow.left),
        grown(shadow.right),
        grown(shadow.top),
        grown(shadow.bottom),
    ) else {
        return (rect, (0, 0));
    };
    let (Some(width), Some(height)) = (
        rect.width
            .checked_sub(left)
            .and_then(|n| n.checked_sub(right)),
        rect.height
            .checked_sub(top)
            .and_then(|n| n.checked_sub(bottom)),
    ) else {
        return (rect, (0, 0));
    };
    let offset = (i16::try_from(left), i16::try_from(top));
    match offset {
        (Ok(x), Ok(y)) if width > 0 && height > 0 => (
            Extents {
                x: rect.x + left,
                y: rect.y + top,
                width,
                height,
            },
            (x, y),
        ),
        _ => (rect, (0, 0)),
    }
}

fn survey(ctx: &WindowContext) -> Result<Geometry, AdapterError> {
    let grabber = ctx.env.grabber;
    let client = grabber
        .describe(ctx.xid, ctx.deadline)
        .map_err(|error| grab_failure(error, ctx.deadline))?;
    if !matches!(client.map_state, MapState::Viewable) {
        return Err(not_viewable(ctx.compositing));
    }
    let frame = match client.parent {
        Some(parent) => {
            let described = grabber
                .describe(parent, ctx.deadline)
                .map_err(|error| grab_failure(error, ctx.deadline))?;
            Some((parent, described.rect))
        }
        None => None,
    };
    let (area, inset) = inside_shadow(client.rect, client.shadow);
    Ok(Geometry {
        client: area,
        inset,
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

fn target_of(geometry: &Geometry, unit: Unit, xid: u32) -> (u32, Extents, (i16, i16)) {
    match (unit, geometry.frame_id, geometry.frame) {
        (Unit::Frame, Some(id), Some(rect)) => (id, rect, (0, 0)),
        _ => (xid, geometry.client, geometry.inset),
    }
}

fn grab_rect_of(rect: Extents, inset: (i16, i16)) -> Result<GrabRect, AdapterError> {
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
        x: inset.0,
        y: inset.1,
        width,
        height,
    })
}

enum Clearance {
    Clear,
    Hidden,
}

fn clearance(ctx: &WindowContext, rect: Extents) -> Clearance {
    if ctx.compositing {
        return Clearance::Clear;
    }
    let read = ctx.env.stacking.read_within(ctx.xid, ctx.deadline);
    match cover(&read, ctx.xid, rect) {
        Cover::Clear => Clearance::Clear,
        Cover::Covered | Cover::Unknown => Clearance::Hidden,
    }
}

fn changed() -> AdapterError {
    AdapterError::stale_ref_because("The window changed while it was being captured")
        .with_suggestion("Run 'list-windows' to refresh window identifiers, then retry")
}

fn recheck(
    ctx: &WindowContext,
    before: &Geometry,
    plain_over: Option<Extents>,
) -> Result<(), AdapterError> {
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
    let Some(rect) = plain_over else {
        return Ok(());
    };
    match clearance(ctx, rect) {
        Clearance::Clear => Ok(()),
        Clearance::Hidden => Err(covered()),
    }
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
    if (env.stopped)(window.pid.get()) {
        return Err(stopped(window));
    }
    let resolved = env.lookup.resolve(window, deadline)?;
    let xid = match xid_or_error(env.lookup.xid(&resolved, deadline)) {
        Err(_) if (env.stopped)(window.pid.get()) => return Err(stopped(window)),
        outcome => outcome?,
    };
    ensure_budget(deadline)?;
    let compositing = env
        .grabber
        .compositing(deadline)
        .map_err(|error| grab_failure(error, deadline))?;
    let ctx = WindowContext {
        env,
        window: resolved,
        xid,
        compositing,
        deadline,
    };
    let before = survey(&ctx)?;
    let unit = unit_of(&before, ctx.window.bounds, strict_unit)?;
    let (drawable, rect, inset) = target_of(&before, unit, xid);
    let area = grab_rect_of(rect, inset)?;
    ensure_budget(deadline)?;
    let plain = matches!(clearance(&ctx, rect), Clearance::Clear);
    let raw = if plain {
        env.grabber.grab(drawable, area, deadline)
    } else {
        env.grabber.grab_redirected(drawable, area, deadline)
    };
    let raw = match raw {
        Ok(raw) => raw,
        Err(GrabError::Gone) => return Err(grab_failure(GrabError::Gone, deadline)),
        Err(_) if !plain => return Err(covered()),
        Err(error) => return Err(grab_failure(error, deadline)),
    };
    recheck(&ctx, &before, plain.then_some(rect))?;
    let displays = env.lookup.displays(deadline).unwrap_or_default();
    let scale = scale_for(&displays, ctx.window.bounds.unwrap_or_else(|| rect.into()));
    let (width, height) = (raw.width, raw.height);
    let rgb = (env.decode)(&raw)?;
    drop(raw);
    (env.encode)(&rgb, width, height, scale)
}

#[cfg(test)]
#[path = "window_world_tests.rs"]
mod world;

#[cfg(test)]
#[path = "window_fakes_tests.rs"]
pub(crate) mod fakes;

#[cfg(test)]
#[path = "window_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "window_guard_tests.rs"]
mod guard_tests;
