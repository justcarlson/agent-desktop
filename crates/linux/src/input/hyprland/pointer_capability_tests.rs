use agent_desktop_core::{Action, AdapterError, Direction, DragParams, ErrorCode, KeyCombo, Point};

use super::{capability_refusal, hover_ref_refused, hover_refusal, ref_gesture_refusal};
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

#[test]
fn a_bound_instance_refuses_a_hover_by_ref_with_hover_unsupported() {
    let refusal = hover_refusal(false, Ok(Backend::Wayland), || Ok(true))
        .unwrap_or_else(|| panic!("a bound instance must refuse a hover by ref"));
    let details = refusal.details.unwrap_or_default();

    assert_eq!(refusal.code, ErrorCode::ActionNotSupported);
    assert_eq!(details["cause"], "hover_unsupported");
    assert_eq!(details["physical_delivery_started"], false);
    assert_eq!(
        refusal.disposition,
        agent_desktop_core::DeliverySemantics::not_delivered()
    );
}

#[test]
fn a_tray_item_keeps_its_hover_on_a_bound_instance() {
    let mut asked = false;
    let refusal = hover_refusal(true, Ok(Backend::Wayland), || {
        asked = true;
        Ok(true)
    });

    assert!(refusal.is_none());
    assert!(!asked);
}

#[test]
fn an_unbound_or_x11_session_leaves_the_hover_by_ref_to_the_pointer_gate() {
    assert!(hover_refusal(false, Ok(Backend::Wayland), || Ok(false)).is_none());
    assert!(
        hover_refusal(false, Ok(Backend::Wayland), || {
            Err(AdapterError::internal("mismatch"))
        })
        .is_none()
    );
    assert!(hover_refusal(false, Ok(Backend::X11(":0".into())), || Ok(true)).is_none());
}

fn asked(action: &Action) -> &'static str {
    let mut asked = "none";
    let hover = || {
        asked = "hover";
        None
    };
    ref_gesture_refusal(action, hover, || None);
    if asked != "none" {
        return asked;
    }
    let receiver = || {
        asked = "receiver";
        None
    };
    ref_gesture_refusal(action, || None, receiver);
    asked
}

#[test]
fn every_ref_pointer_gesture_asks_the_receiver_gate_and_hover_asks_its_own() {
    let drag = Action::Drag(DragParams {
        from: Point { x: 0.0, y: 0.0 },
        to: Point { x: 1.0, y: 1.0 },
        duration_ms: None,
        drop_delay_ms: None,
    });
    for action in [
        Action::Click,
        Action::DoubleClick,
        Action::TripleClick,
        Action::RightClick,
        Action::Scroll(Direction::Down, 3),
        drag,
    ] {
        assert_eq!(asked(&action), "receiver", "{}", action.name());
    }
    assert_eq!(asked(&Action::Hover), "hover");
}

#[test]
fn keyboard_and_semantic_actions_are_never_refused_as_pointer_gestures() {
    for action in [
        Action::TypeText("hi".into()),
        Action::PressKey(KeyCombo {
            key: "a".into(),
            modifiers: vec![],
        }),
        Action::ScrollTo,
        Action::SetFocus,
    ] {
        assert_eq!(asked(&action), "none", "{}", action.name());
    }
}
