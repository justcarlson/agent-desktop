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
use super::native_receiver::refuse_native_receiver;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::cursor_context::CursorContext;
use crate::system::hyprland::placement::placement_read::{
    place_target, placed_bounds, popup_refusal,
};
use crate::system::hyprland::receiver::receiver_hit::read_hit;
use crate::system::hyprland::receiver::receiver_read::prove_receiver;
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

/// The refusal a pointer action on a bound element gets at once when the
/// element has no bounds because its frame is a popup surface; `Some(None)`
/// for any other bound element and for a frame that cannot be read, and
/// `None` when the element is not bound.
pub(crate) fn bound_position_refusal(
    bus: &SharedBus,
    handle: &NativeHandle,
    deadline: Deadline,
) -> Option<Option<AdapterError>> {
    let (_, element) = bound_target(handle).ok().flatten()?;
    Some(popup_refusal(bus, element, deadline).ok().flatten())
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
        read_hit(&binding, placed.address, point, deadline)
    })
    .map(Some)
}

/// Proves that `point` reaches the element before a drag by ref uses it as
/// an endpoint: the element is placed on its shown toplevel and the
/// compositor's stack, read with the agent's cursors hidden, must show no
/// cover, and its window must be an XWayland client. Anything less refuses
/// before any event.
pub(crate) fn prove_bound_point(
    bus: &SharedBus,
    handle: &NativeHandle,
    point: &Point,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    let Some((binding, element)) = bound_target(handle)? else {
        return Ok(());
    };
    let placed = place_target(&binding, bus, element, Some(point), deadline)?;
    refuse_native_receiver(&binding, placed.address, deadline)?;
    let mut hider = crate::system::hyprland::cursor_hider(&CursorContext { binding: &binding });
    with_cursors_hidden(hider.as_mut(), deadline, || {
        prove_receiver(&binding, placed.address, &placed.at, deadline)
    })
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
