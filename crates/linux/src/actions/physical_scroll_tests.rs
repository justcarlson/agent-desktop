use agent_desktop_core::{Direction, ErrorCode, InteractionPolicy};

use super::{scroll_steps, wheel_deltas};
use crate::actions::physical_target::tests::fake::{FakeWorld, check_of, deadline, element};

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

#[test]
fn every_direction_reaches_the_x_button_that_scrolls_that_way() {
    use crate::input::keymap::Keymap;
    use crate::input::step::Step;
    use crate::input::wheel_plan::{
        BUTTON_WHEEL_DOWN, BUTTON_WHEEL_LEFT, BUTTON_WHEEL_RIGHT, BUTTON_WHEEL_UP, plan_wheel,
    };
    let at = agent_desktop_core::Point { x: 5.0, y: 5.0 };
    for (direction, button) in [
        (Direction::Up, BUTTON_WHEEL_UP),
        (Direction::Down, BUTTON_WHEEL_DOWN),
        (Direction::Left, BUTTON_WHEEL_LEFT),
        (Direction::Right, BUTTON_WHEEL_RIGHT),
    ] {
        let (dx, dy) = wheel_deltas(&direction, 1);
        let plans = plan_wheel(&Keymap::sample(), at.clone(), dx, dy, &[]).expect("plan");
        let pressed: Vec<u8> = plans
            .iter()
            .flat_map(|plan| &plan.steps)
            .filter_map(|step| match step {
                Step::Button { code, down: true } => Some(*code),
                _ => None,
            })
            .collect();
        assert_eq!(pressed, vec![button], "{direction:?}");
    }
}
