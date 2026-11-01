use agent_desktop_core::{AdapterError, ErrorCode};

use super::{capability_refusal, hover_ref_refused};
use crate::input::backend::Backend;

#[test]
fn a_bound_hyprland_instance_opens_the_wayland_session_to_pointer_input() {
    assert!(capability_refusal(Ok(Backend::Wayland), || Ok(true)).is_none());
}

#[test]
fn a_wayland_session_without_a_bound_instance_refuses_pointer_input() {
    let refusal = capability_refusal(Ok(Backend::Wayland), || Ok(false))
        .unwrap_or_else(|| panic!("an unbound Wayland session must refuse"));

    assert_eq!(refusal.code, ErrorCode::ActionNotSupported);
    assert_eq!(
        refusal.details.unwrap_or_default()["cause"],
        "wayland_session"
    );
}

#[test]
fn a_failed_bind_is_the_refusal() {
    let refusal = capability_refusal(Ok(Backend::Wayland), || {
        Err(AdapterError::new(
            ErrorCode::PlatformNotSupported,
            "mismatch",
        ))
    })
    .unwrap_or_else(|| panic!("a failed bind must refuse"));

    assert_eq!(refusal.code, ErrorCode::PlatformNotSupported);
}

#[test]
fn a_session_without_a_known_backend_leaves_the_refusal_to_delivery() {
    let mut asked = false;
    let refusal = capability_refusal(Err(AdapterError::internal("no display")), || {
        asked = true;
        Ok(false)
    });

    assert!(refusal.is_none());
    assert!(!asked);
}

#[test]
fn hover_by_ref_is_refused_before_any_event() {
    let refusal = hover_ref_refused();
    let details = refusal.details.unwrap_or_default();

    assert_eq!(refusal.code, ErrorCode::ActionNotSupported);
    assert_eq!(details["cause"], "hover_unsupported");
    assert_eq!(details["physical_delivery_started"], false);
}
