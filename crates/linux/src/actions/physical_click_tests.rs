use agent_desktop_core::{ErrorCode, InteractionPolicy, Point};

use super::{ClickSpec, click_steps};
use crate::actions::physical_target::tests::fake::{
    FakeWorld, check_of, deadline, element, not_delivered,
};

#[test]
fn a_click_without_a_verified_point_is_aimed_at_the_centre_of_the_bounds() {
    let world = FakeWorld::x11();
    click_steps(
        &world,
        &element(),
        ClickSpec::left(2, None),
        InteractionPolicy::headed(),
        deadline(),
    )
    .unwrap();
    assert!(
        world
            .entries()
            .contains(&"plan_click 120,210 Left 2".to_owned())
    );
    assert_eq!(world.count("sent"), 1);
}

#[test]
fn a_verified_point_is_used_and_the_bounds_are_not_read() {
    let world = FakeWorld::x11();
    let point = Point { x: 7.0, y: 9.0 };
    click_steps(
        &world,
        &element(),
        ClickSpec::right(Some(point)),
        InteractionPolicy::headed(),
        deadline(),
    )
    .unwrap();
    assert!(
        world
            .entries()
            .contains(&"plan_click 7,9 Right 1".to_owned())
    );
    assert_eq!(world.count("bounds"), 0);
}

#[test]
fn bounds_that_cannot_be_established_refuse_with_no_event() {
    for bounds in [
        None,
        Some(agent_desktop_core::Rect {
            x: 5.0,
            y: 5.0,
            width: 0.0,
            height: 10.0,
        }),
    ] {
        let world = FakeWorld::x11().with_bounds(bounds);
        let error = click_steps(
            &world,
            &element(),
            ClickSpec::left(1, None),
            InteractionPolicy::headed(),
            deadline(),
        )
        .unwrap_err();
        assert_eq!(check_of(&error).as_deref(), Some("geometry"));
        assert!(not_delivered(&error));
        assert!(world.sent_nothing());
    }
}

#[test]
fn focus_only_policy_denies_a_click() {
    let world = FakeWorld::x11();
    let error = click_steps(
        &world,
        &element(),
        ClickSpec::left(1, None),
        InteractionPolicy::focus_fallback(),
        deadline(),
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::PolicyDenied);
    assert!(world.sent_nothing());
}

#[test]
fn a_click_does_not_wait_for_the_frame_to_become_active() {
    let world = FakeWorld::x11().active_script(&[Some(false)]);
    click_steps(
        &world,
        &element(),
        ClickSpec::left(1, None),
        InteractionPolicy::headed(),
        deadline(),
    )
    .unwrap();
    assert_eq!(world.count("read_active"), 0);
    assert_eq!(world.count("sent"), 1);
}

#[test]
fn on_wayland_a_click_is_refused_before_anything_is_computed() {
    let world = FakeWorld::wayland();
    let error = click_steps(
        &world,
        &element(),
        ClickSpec::left(1, None),
        InteractionPolicy::headed(),
        deadline(),
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    assert_eq!(error.message, "pointer_refusal click");
    assert_eq!(world.count("bounds"), 0);
    assert_eq!(world.count("plan_click"), 0);
    assert!(world.sent_nothing());
}

#[test]
fn the_step_is_a_physical_unverified_click() {
    let world = FakeWorld::x11();
    let steps = click_steps(
        &world,
        &element(),
        ClickSpec::left(1, None),
        InteractionPolicy::headed(),
        deadline(),
    )
    .unwrap();
    assert_eq!(steps[0].label(), "XTEST.click");
    assert_eq!(steps[0].verified(), Some(false));
}
