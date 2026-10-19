use agent_desktop_core::{Direction, ErrorCode, InteractionPolicy};

use super::{scroll_steps, wheel_deltas};
use crate::actions::physical_target::tests::fake::{FakeWorld, check_of, deadline, element};

#[test]
fn down_turns_the_wheel_toward_the_user_and_up_away_from_them() {
    assert_eq!(wheel_deltas(&Direction::Down, 3), (0.0, -3.0));
    assert_eq!(wheel_deltas(&Direction::Up, 3), (0.0, 3.0));
}

#[test]
fn left_and_right_use_the_horizontal_axis() {
    assert_eq!(wheel_deltas(&Direction::Left, 2), (-2.0, 0.0));
    assert_eq!(wheel_deltas(&Direction::Right, 2), (2.0, 0.0));
}

#[test]
fn the_wheel_is_aimed_at_the_centre_of_the_targets_own_bounds() {
    let world = FakeWorld::x11();
    scroll_steps(
        &world,
        &element(),
        (0.0, -4.0),
        InteractionPolicy::headed(),
        deadline(),
    )
    .unwrap();
    assert!(
        world
            .entries()
            .contains(&"plan_wheel 120,210 0,-4".to_owned())
    );
    assert_eq!(world.count("sent"), 1);
}

#[test]
fn bounds_that_cannot_be_established_refuse_with_no_event() {
    let world = FakeWorld::x11().with_bounds(None);
    let error = scroll_steps(
        &world,
        &element(),
        (0.0, -1.0),
        InteractionPolicy::headed(),
        deadline(),
    )
    .unwrap_err();
    assert_eq!(check_of(&error).as_deref(), Some("geometry"));
    assert!(world.sent_nothing());
}

#[test]
fn the_wheel_needs_the_cursor_and_focus_policy() {
    for policy in [
        InteractionPolicy::headless(),
        InteractionPolicy::focus_fallback(),
    ] {
        let world = FakeWorld::x11();
        let error = scroll_steps(&world, &element(), (0.0, -1.0), policy, deadline()).unwrap_err();
        assert_eq!(error.code, ErrorCode::PolicyDenied);
        assert!(world.sent_nothing());
    }
}

#[test]
fn on_wayland_the_wheel_is_refused_before_anything_is_computed() {
    let world = FakeWorld::wayland();
    let error = scroll_steps(
        &world,
        &element(),
        (0.0, -1.0),
        InteractionPolicy::headed(),
        deadline(),
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    assert_eq!(error.message, "pointer_refusal scroll");
    assert_eq!(world.count("bounds"), 0);
    assert!(world.sent_nothing());
}
