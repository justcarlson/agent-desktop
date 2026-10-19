#[path = "physical_fake.rs"]
pub(crate) mod fake;

use agent_desktop_core::{Action, ActionRequest, DragParams, ErrorCode, KeyCombo, Point};

use super::{aim_point, deliver_in, ensure_headed_click_policy, ensure_keyboard_policy};
use agent_desktop_core::InteractionPolicy;
use fake::{FakeWorld, deadline, element, not_delivered};

fn combo() -> KeyCombo {
    KeyCombo {
        key: "a".to_owned(),
        modifiers: Vec::new(),
    }
}

fn run(
    world: &FakeWorld,
    action: Action,
    with_element: bool,
) -> Result<(), agent_desktop_core::AdapterError> {
    let request = ActionRequest::headed(action.clone());
    let target = element();
    deliver_in(
        world,
        &action,
        with_element.then_some(&target),
        &request,
        deadline(),
    )
    .map(|_| ())
}

#[test]
fn hover_and_drag_are_rejected_at_the_adapter_with_nothing_sent() {
    let drag = Action::Drag(DragParams {
        from: Point { x: 0.0, y: 0.0 },
        to: Point { x: 1.0, y: 1.0 },
        duration_ms: None,
        drop_delay_ms: None,
    });
    for action in [Action::Hover, drag] {
        let world = FakeWorld::x11();
        let error = run(&world, action, true).unwrap_err();
        assert_eq!(error.code, ErrorCode::ActionNotSupported);
        assert!(error.message.contains("adapter-level handling"));
        assert!(not_delivered(&error));
        assert!(world.sent_nothing());
        assert!(world.entries().is_empty());
    }
}

#[test]
fn held_key_edges_are_rejected_at_the_adapter() {
    for action in [Action::KeyDown(combo()), Action::KeyUp(combo())] {
        let world = FakeWorld::x11();
        let error = run(&world, action, false).unwrap_err();
        assert_eq!(error.code, ErrorCode::ActionNotSupported);
        assert!(world.sent_nothing());
    }
}

#[test]
fn an_action_with_no_physical_leg_is_not_supported_with_nothing_sent() {
    for action in [
        Action::SetValue("x".to_owned()),
        Action::Toggle,
        Action::Clear,
    ] {
        let world = FakeWorld::x11();
        let error = run(&world, action, true).unwrap_err();
        assert_eq!(error.code, ErrorCode::ActionNotSupported);
        assert!(world.sent_nothing());
    }
}

#[test]
fn a_ref_action_without_an_element_is_invalid() {
    let world = FakeWorld::x11();
    let error = run(&world, Action::Click, false).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidArgs);
    assert!(world.sent_nothing());
}

#[test]
fn a_null_handle_press_sends_with_no_gate_even_under_headless_policy() {
    let world = FakeWorld::x11().active_script(&[Some(false)]);
    let action = Action::PressKey(combo());
    let request = ActionRequest::headless(action.clone());
    let result = deliver_in(&world, &action, None, &request, deadline()).unwrap();
    assert_eq!(result.steps[0].label(), "XTEST.press_key");
    assert_eq!(world.count("read_active"), 0);
    assert_eq!(world.count("sent"), 1);
}

#[test]
fn each_click_kind_reaches_the_pointer_leg_with_its_button_and_count() {
    let cases = [
        (Action::Click, "Left 1"),
        (Action::DoubleClick, "Left 2"),
        (Action::TripleClick, "Left 3"),
        (Action::RightClick, "Right 1"),
    ];
    for (action, expected) in cases {
        let world = FakeWorld::x11();
        run(&world, action, true).unwrap();
        let wanted = format!("plan_click 120,210 {expected}");
        assert!(world.entries().contains(&wanted), "{wanted}");
    }
}

#[test]
fn a_scroll_action_reaches_the_wheel_leg_with_the_signed_notches() {
    let world = FakeWorld::x11();
    run(
        &world,
        Action::Scroll(agent_desktop_core::Direction::Down, 3),
        true,
    )
    .unwrap();
    assert!(
        world
            .entries()
            .contains(&"plan_wheel 120,210 0,-3".to_owned())
    );
}

#[test]
fn type_and_press_on_an_element_reach_the_keyboard_leg() {
    let world = FakeWorld::x11();
    run(&world, Action::TypeText("hi".to_owned()), true).unwrap();
    run(&world, Action::PressKey(combo()), true).unwrap();
    assert_eq!(world.count("plan_text"), 1);
    assert_eq!(world.count("plan_combo"), 1);
}

#[test]
fn the_policies_split_keys_from_the_pointer() {
    assert!(ensure_keyboard_policy(InteractionPolicy::headless()).is_err());
    assert!(ensure_keyboard_policy(InteractionPolicy::focus_fallback()).is_ok());
    assert!(ensure_headed_click_policy(InteractionPolicy::focus_fallback()).is_err());
    assert!(ensure_headed_click_policy(InteractionPolicy::headed()).is_ok());
}

#[test]
fn a_non_finite_verified_point_is_refused_before_it_is_aimed_at() {
    let world = FakeWorld::x11();
    let point = Point {
        x: f64::NAN,
        y: 1.0,
    };
    let error = aim_point(&world, &element(), Some(&point), deadline()).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidArgs);
}
