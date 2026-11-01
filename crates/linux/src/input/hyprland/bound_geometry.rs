//! Where a ref element is and what a point over it reaches on the bound
//! Hyprland instance. Toolkit and X11 geometry cannot place a Wayland
//! window, so live bounds, pointer observations and hit tests answer from
//! the compositor instead. Every answer is `None` when no Hyprland is bound,
//! and for shell elements, which keep their own path.

use agent_desktop_core::{
    AdapterError, Deadline, ErrorCode, HitTestResult, NativeHandle, Point, Rect,
};

use super::cursor_guard::with_cursors_hidden;
use super::hypr_route::bound_route;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::cursor_context::CursorContext;
use crate::system::hyprland::placement::placement_read::{place_target, placed_bounds};
use crate::system::hyprland::receiver::receiver_hit::read_hit;
use crate::system::{SharedBus, shell};
use crate::tree::{AtspiElement, atspi_element};

fn bound_target(
    handle: &NativeHandle,
) -> Result<Option<(HyprlandBinding, &AtspiElement)>, AdapterError> {
    let Ok(element) = atspi_element(handle) else {
        return Ok(None);
    };
    if shell::is_shell_element(element) {
        return Ok(None);
    }
    Ok(bound_route()?.map(|binding| (binding, element)))
}

/// Whether the element takes its geometry from the bound Hyprland instance,
/// which names its toplevel without the X11 twin rule.
pub(crate) fn is_bound(handle: &NativeHandle) -> bool {
    matches!(bound_target(handle), Ok(Some(_)))
}

/// The element's global layout rectangle, `Some(None)` when the compositor
/// cannot place it.
pub(crate) fn bound_bounds(
    bus: &SharedBus,
    handle: &NativeHandle,
    deadline: Deadline,
) -> Result<Option<Option<Rect>>, AdapterError> {
    let Some((binding, element)) = bound_target(handle)? else {
        return Ok(None);
    };
    unplaced_as(
        placed_bounds(&binding, bus, element, deadline).map(Some),
        None,
    )
    .map(Some)
}

/// Whether `point` reaches the element, read from the compositor's stack
/// with the agent's cursors hidden first.
pub(crate) fn bound_hit(
    bus: &SharedBus,
    handle: &NativeHandle,
    point: &Point,
    deadline: Deadline,
) -> Result<Option<HitTestResult>, AdapterError> {
    let Some((binding, element)) = bound_target(handle)? else {
        return Ok(None);
    };
    let placed = unplaced_as(
        place_target(&binding, bus, element, Some(point), deadline).map(Some),
        None,
    )?;
    let Some(placed) = placed else {
        return Ok(Some(HitTestResult::Unknown));
    };
    let mut hider = crate::system::hyprland::cursor_hider(&CursorContext { binding: &binding });
    with_cursors_hidden(hider.as_mut(), deadline, || {
        Ok(read_hit(&binding, placed.address, point, deadline))
    })
    .map(Some)
}

/// A timeout stays an error; any other refusal to place the element
/// answers `unplaced`.
pub(crate) fn unplaced_as<T>(
    result: Result<T, AdapterError>,
    unplaced: T,
) -> Result<T, AdapterError> {
    match result {
        Err(error) if error.code == ErrorCode::Timeout => Err(error),
        Err(_) => Ok(unplaced),
        Ok(value) => Ok(value),
    }
}

#[cfg(test)]
#[path = "bound_geometry_tests.rs"]
mod tests;
