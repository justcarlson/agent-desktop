use super::*;
use agent_desktop_core::{Deadline, Modifier, MouseButton, MouseEvent, MouseEventKind, Point};

#[test]
fn the_error_carries_all_three_details_and_the_not_delivered_disposition() {
    let error = reject_standalone("mouse_event down");
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    assert_eq!(
        error.message,
        "Standalone mouse_event down is unavailable in stateless mode"
    );
    let details = error.details.as_ref().unwrap();
    assert_eq!(details["raw_input_emitted"], false);
    assert_eq!(details["requires_daemon_owned_transaction"], true);
    assert_eq!(
        error.suggestion.as_deref(),
        Some(
            "Use the atomic 'press', 'mouse-click' or 'drag'; spanning holds require a daemon-owned session that can release input after disconnect"
        )
    );
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
}

#[test]
fn standalone_edges_refuse_before_backend_lookup() {
    let deadline = Deadline::detached_after(2_000).unwrap();
    for (kind, direction) in [(MouseEventKind::Down, "down"), (MouseEventKind::Up, "up")] {
        let event = MouseEvent {
            kind: kind.clone(),
            point: Point { x: 10.0, y: 10.0 },
            button: MouseButton::Left,
            modifiers: vec![Modifier::Ctrl],
        };
        let error =
            crate::input::deliver::mouse_event(&event, deadline).expect_err("held edge refusal");
        assert_eq!(error.code, ErrorCode::ActionNotSupported);
        assert_eq!(
            error.message,
            format!("Standalone mouse_event {direction} is unavailable in stateless mode")
        );
        assert_eq!(error.details.as_ref().unwrap()["raw_input_emitted"], false);
        assert_eq!(
            error.details.as_ref().unwrap()["requires_daemon_owned_transaction"],
            true
        );
        assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    }
}
