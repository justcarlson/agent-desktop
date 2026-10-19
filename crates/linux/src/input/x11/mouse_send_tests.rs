use agent_desktop_core::{Deadline, ErrorCode, MouseButton, MouseEvent, MouseEventKind, Point};

use super::mouse_event;
use crate::input::standalone::reject_standalone;

fn event(kind: MouseEventKind) -> MouseEvent {
    MouseEvent {
        kind,
        point: Point { x: 10.0, y: 10.0 },
        button: MouseButton::Left,
        modifiers: Vec::new(),
    }
}

fn deadline() -> Deadline {
    Deadline::detached_after(2_000).expect("deadline")
}

#[test]
fn held_edges_are_refused_before_any_connection_is_made() {
    for kind in [MouseEventKind::Down, MouseEventKind::Up] {
        let error =
            mouse_event("unused-display-for-test", &event(kind), deadline()).expect_err("refused");

        assert_eq!(error.code, reject_standalone("mouse_event").code);
        assert_ne!(error.code, ErrorCode::ActionFailed);
    }
}
