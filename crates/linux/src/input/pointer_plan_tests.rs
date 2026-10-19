use super::*;
use agent_desktop_core::ErrorCode;

fn at() -> Point {
    Point { x: 100.4, y: 200.6 }
}

fn click(count: u32, mods: &[Modifier]) -> Result<Plan, AdapterError> {
    plan_click(&Keymap::sample(), at(), &MouseButton::Left, count, mods)
}

fn button(down: bool) -> Step {
    Step::Button { code: 1, down }
}

#[test]
fn a_single_click_moves_presses_holds_and_releases() {
    let plan = click(1, &[]).unwrap();
    assert_eq!(
        plan.steps,
        vec![
            Step::MoveTo { x: 100, y: 201 },
            button(true),
            Step::Pause(CLICK_HOLD),
            button(false),
        ]
    );
    assert_eq!(plan.release_origin, Some((100, 201)));
}

#[test]
fn a_double_click_has_the_gap_between_pairs_and_only_the_hold_inside_one() {
    let plan = click(2, &[]).unwrap();
    assert_eq!(
        plan.steps[1..],
        [
            button(true),
            Step::Pause(CLICK_HOLD),
            button(false),
            Step::Pause(CLICK_GAP),
            button(true),
            Step::Pause(CLICK_HOLD),
            button(false),
        ]
    );
}

#[test]
fn a_click_count_of_zero_or_above_the_cap_is_invalid_args() {
    for count in [0, agent_desktop_core::MAX_MOUSE_CLICK_COUNT + 1] {
        assert_eq!(click(count, &[]).unwrap_err().code, ErrorCode::InvalidArgs);
    }
    assert!(click(agent_desktop_core::MAX_MOUSE_CLICK_COUNT, &[]).is_ok());
}

#[test]
fn buttons_map_to_the_x11_codes() {
    for (button, code) in [
        (MouseButton::Left, 1),
        (MouseButton::Middle, 2),
        (MouseButton::Right, 3),
    ] {
        let plan = plan_click(&Keymap::sample(), at(), &button, 1, &[]).unwrap();
        assert_eq!(plan.steps[1], Step::Button { code, down: true });
    }
}

#[test]
fn modifiers_wrap_the_gesture_and_release_in_reverse() {
    let plan = click(1, &[Modifier::Ctrl, Modifier::Shift]).unwrap();
    let key = |code, down| Step::Key { code, down };
    assert_eq!(plan.steps[..2], [key(9, true), key(8, true)]);
    let tail = &plan.steps[plan.steps.len() - 2..];
    assert_eq!(tail, [key(8, false), key(9, false)]);
}

#[test]
fn a_modifier_the_keyboard_lacks_fails_before_any_step() {
    assert!(click(1, &[Modifier::Meta]).is_ok());
    let bare = Keymap::new(8, 2, vec![1, 2], 0, Vec::new());
    let error = plan_click(&bare, at(), &MouseButton::Left, 1, &[Modifier::Ctrl]).unwrap_err();
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
}

#[test]
fn a_point_that_is_not_finite_is_invalid_args() {
    let error = plan_click(
        &Keymap::sample(),
        Point {
            x: f64::NAN,
            y: 0.0,
        },
        &MouseButton::Left,
        1,
        &[],
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidArgs);
}

#[test]
fn a_negative_coordinate_is_planned_and_left_to_the_geometry_check() {
    let plan = plan_move(Point { x: -84.0, y: 10.0 }).unwrap();
    assert_eq!(plan.steps, vec![Step::MoveTo { x: -84, y: 10 }]);
    assert_eq!(plan.release_origin, None);
}
