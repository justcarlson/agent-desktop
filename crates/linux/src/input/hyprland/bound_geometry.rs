//! Where a ref element is and what a point over it reaches on the bound
//! Hyprland instance. Toolkit and X11 geometry cannot place a Wayland
//! window, so live bounds, pointer observations and hit tests answer from
//! the compositor instead. Every answer is `None` when no Hyprland is bound,
//! and for shell elements, which keep their own path.

use agent_desktop_core::{
    AdapterError, Deadline, ErrorCode, HitTestResult, LiveElement, NativeHandle, Point, Rect,
    RefEntry,
};

use super::cursor_guard::with_cursors_hidden;
use super::hypr_route::bound_route;
use super::native_receiver::{entry_receiver_refusal, refuse_unproven_receiver};
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::cursor_context::CursorContext;
use crate::system::hyprland::cursor_hider::CursorHider;
use crate::system::hyprland::ipc::bound_request::bound_request;
use crate::system::hyprland::placement::placed_target::PlacedTarget;
use crate::system::hyprland::placement::placement_read::{
    place_target, placed_bounds, popup_refusal,
};
use crate::system::hyprland::receiver::receiver_hit::read_hit;
use crate::system::hyprland::receiver::receiver_read::prove_receiver;
use crate::system::{SharedBus, shell, tray};
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

/// `live` with the bound instance's answer for its bounds: the rectangle the
/// compositor places, or none when it cannot place the element.
pub(crate) fn with_bound_bounds(mut live: LiveElement, placed: Option<Rect>) -> LiveElement {
    live.bounds = placed;
    live
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
/// an endpoint: the element is placed on its shown toplevel, the receiver
/// gate must pass, and the compositor's stack, read with the agent's
/// cursors hidden, must show no cover. Anything less refuses before any
/// event, and the gate refuses every target today.
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
    let mut hider = crate::system::hyprland::cursor_hider(&CursorContext { binding: &binding });
    prove_placed_point(&binding, &placed, hider.as_mut(), deadline)
}

/// The refusal for a ref pointer gesture on the bound Hyprland instance,
/// read from the saved entry before core resolves the element, activates its
/// window or moves the cursor. No headed ref pointer gesture delivers there,
/// so every bound answer refuses, whether or not the element still resolves.
/// `None` when no Hyprland is bound and for tray and shell refs, which keep
/// their own path.
pub(crate) fn ref_receiver_refusal(entry: &RefEntry, deadline: Deadline) -> Option<AdapterError> {
    if tray::owns(entry) {
        return None;
    }
    let binding = bound_route().ok().flatten()?;
    let pid = entry.process.pid.get();
    if shell::is_shell_process(pid) {
        return None;
    }
    let clients = bound_request(&binding, "j/clients", deadline).ok();
    Some(entry_receiver_refusal(clients.as_deref(), pid))
}

/// [`prove_bound_point`] for a placed endpoint and the hider that hides the
/// agent's cursors around the stack read. The gate refuses before the hide.
pub(crate) fn prove_placed_point(
    binding: &HyprlandBinding,
    placed: &PlacedTarget,
    hider: &mut dyn CursorHider,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    refuse_unproven_receiver(binding, placed.address, deadline)?;
    with_cursors_hidden(hider, deadline, || {
        prove_receiver(binding, placed.address, &placed.at, deadline)
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
