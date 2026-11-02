//! Whether this session can take pointer input at all, answered before core
//! resolves any pointer target. Only a bound Hyprland instance opens the
//! Wayland session to pointer input.

use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode, RefEntry};
use serde_json::json;

use super::hypr_route::bound_route;
use crate::input::backend::{Backend, backend};
use crate::input::wayland::keysym_send::pointer_refusal;

/// The refusal for this session: none on X11 or on a bound Hyprland
/// instance, the Wayland session refusal on any other Wayland session, and
/// the bind error when the bind itself fails.
pub(crate) fn session_pointer_refusal() -> Option<AdapterError> {
    capability_refusal(backend(), || bound_route().map(|route| route.is_some()))
}

/// [`session_pointer_refusal`] for a known backend and bind result.
pub(crate) fn capability_refusal(
    backend: Result<Backend, AdapterError>,
    bound: impl FnOnce() -> Result<bool, AdapterError>,
) -> Option<AdapterError> {
    match backend {
        Ok(Backend::Wayland) => match bound() {
            Ok(true) => None,
            Ok(false) => Some(pointer_refusal("physical pointer input")),
            Err(error) => Some(error),
        },
        _ => None,
    }
}

/// The refusal for a hover by ref, answered from the stored entry before
/// core waits, focuses or resolves anything. A bound Hyprland instance
/// refuses every hover by ref except one on a tray item: the wait would only
/// spend the budget, and on a GTK 4 popup under XWayland it ended `TIMEOUT`
/// before the refusal (probes/linux/FINDINGS.md L45-3).
pub(crate) fn session_hover_refusal(entry: &RefEntry) -> Option<AdapterError> {
    hover_refusal(crate::system::tray::owns(entry), backend(), || {
        bound_route().map(|route| route.is_some())
    })
}

/// [`session_hover_refusal`] for a known tray answer, backend and bind
/// result. Any session that is not bound leaves the answer to the pointer
/// gate, which already refused it or let X11 through.
pub(crate) fn hover_refusal(
    tray: bool,
    backend: Result<Backend, AdapterError>,
    bound: impl FnOnce() -> Result<bool, AdapterError>,
) -> Option<AdapterError> {
    match backend {
        Ok(Backend::Wayland) if !tray => match bound() {
            Ok(true) => Some(hover_ref_refused()),
            _ => None,
        },
        _ => None,
    }
}

/// The refusal for a hover by ref on a bound Hyprland instance. Hover gives
/// the adapter only a point, so no receiver evidence ties it to the ref.
pub(crate) fn hover_ref_refused() -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionNotSupported,
        "Hover by ref has no receiver evidence on Hyprland",
    )
    .with_details(json!({
        "kind": "hyprland_pointer",
        "cause": "hover_unsupported",
        "physical_delivery_started": false,
    }))
    .with_suggestion("Use mouse-move with coordinates, or a ref action such as click")
    .with_disposition(DeliverySemantics::not_delivered())
}

#[cfg(test)]
#[path = "pointer_capability_tests.rs"]
mod tests;
