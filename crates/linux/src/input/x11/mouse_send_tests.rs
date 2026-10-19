use agent_desktop_core::{Deadline, ErrorCode, MouseButton, MouseEvent, MouseEventKind, Point};

use super::{mouse_event, move_plan, pointer_target};
use crate::input::standalone::reject_standalone;
use crate::input::step::Step;

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

#[test]
fn a_move_plans_one_absolute_step_at_the_rounded_point() {
    let plan = move_plan(Point { x: 10.4, y: 20.6 }).expect("plan");

    assert_eq!(plan.steps, vec![Step::MoveTo { x: 10, y: 21 }]);
    assert_eq!(plan.release_origin, None);
}

#[test]
fn a_point_that_is_not_finite_or_not_sane_is_invalid() {
    for bad in [f64::NAN, f64::INFINITY, -2_147_483_648.0, 1.0e9] {
        let error = pointer_target(Point { x: bad, y: 0.0 }).expect_err("invalid");

        assert_eq!(error.code, ErrorCode::InvalidArgs);
    }
}
