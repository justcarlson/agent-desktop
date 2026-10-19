use agent_desktop_core::{ErrorCode, InteractionPolicy, KeyCombo};

use super::{press_key_element_steps, press_key_global_steps, type_text_steps};
use crate::actions::physical_target::tests::fake::{
    FakeWorld, check_of, deadline, element, expired, not_delivered,
};

fn combo() -> KeyCombo {
    KeyCombo {
        key: "a".to_owned(),
        modifiers: Vec::new(),
    }
}

#[test]
fn headless_type_is_denied_by_policy_and_sends_nothing() {
    let world = FakeWorld::x11();
    let error = type_text_steps(
        &world,
        &element(),
        "hi",
        InteractionPolicy::headless(),
        deadline(),
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::PolicyDenied);
    assert!(world.sent_nothing());
    assert_eq!(world.count("read_active"), 0);
}

#[test]
fn headless_press_on_an_element_is_denied_by_policy() {
    let world = FakeWorld::x11();
    let error = press_key_element_steps(
        &world,
        &element(),
        &combo(),
        InteractionPolicy::headless(),
        deadline(),
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::PolicyDenied);
    assert!(world.sent_nothing());
}

#[test]
fn focus_only_policy_lets_keys_through() {
    let world = FakeWorld::x11();
    let steps = type_text_steps(
        &world,
        &element(),
        "hi",
        InteractionPolicy::focus_fallback(),
        deadline(),
    )
    .unwrap();
    assert_eq!(steps[0].label(), "XTEST.type_text");
    assert_eq!(steps[0].verified(), Some(false));
    assert_eq!(world.count("sent"), 1);
}

#[test]
fn a_frame_that_never_reports_active_fails_with_nothing_sent() {
    let world = FakeWorld::x11().active_script(&[Some(false)]);
    let error = type_text_steps(
        &world,
        &element(),
        "hi",
        InteractionPolicy::headed(),
        deadline(),
    )
    .unwrap_err();
    assert_eq!(check_of(&error).as_deref(), Some("window_active"));
    assert!(not_delivered(&error));
    assert!(world.sent_nothing());
}

#[test]
fn a_frame_that_becomes_active_after_two_polls_proceeds() {
    let world = FakeWorld::x11().active_script(&[Some(false), Some(false), Some(true)]);
    type_text_steps(
        &world,
        &element(),
        "hi",
        InteractionPolicy::headed(),
        deadline(),
    )
    .unwrap();
    assert_eq!(world.count("sent"), 1);
}

#[test]
fn a_frame_that_loses_active_before_the_third_chunk_stops_after_two() {
    let world = FakeWorld::x11().with_chunks(3).active_script(&[
        Some(true),
        Some(true),
        Some(true),
        Some(false),
    ]);
    let error = type_text_steps(
        &world,
        &element(),
        "long text",
        InteractionPolicy::headed(),
        deadline(),
    )
    .unwrap_err();
    assert_eq!(check_of(&error).as_deref(), Some("window_active"));
    assert_eq!(world.count("sent"), 2);
}

#[test]
fn the_gate_is_read_before_the_first_key_on_the_press_leg_too() {
    let world = FakeWorld::x11().active_script(&[Some(false)]);
    let error = press_key_element_steps(
        &world,
        &element(),
        &combo(),
        InteractionPolicy::focus_fallback(),
        deadline(),
    )
    .unwrap_err();
    assert_eq!(check_of(&error).as_deref(), Some("window_active"));
    assert!(world.sent_nothing());
}

#[test]
fn a_global_press_sends_with_no_gate_and_no_frame_read() {
    let world = FakeWorld::x11().active_script(&[Some(false)]);
    let steps = press_key_global_steps(&world, &combo(), deadline()).unwrap();
    assert_eq!(steps[0].label(), "XTEST.press_key");
    assert_eq!(world.count("read_active"), 0);
    assert_eq!(world.count("sent"), 1);
}

#[test]
fn an_expired_deadline_fails_before_any_event() {
    let world = FakeWorld::x11();
    let error = press_key_global_steps(&world, &combo(), expired()).unwrap_err();
    assert_eq!(error.code, ErrorCode::Timeout);
    assert!(not_delivered(&error));
    assert!(world.sent_nothing());
}

#[test]
fn wayland_keys_go_through_the_keysym_sender_and_are_gated_after_the_lead_key() {
    let world = FakeWorld::wayland().with_chunks(2);
    let steps = type_text_steps(
        &world,
        &element(),
        "hi",
        InteractionPolicy::focus_fallback(),
        deadline(),
    )
    .unwrap();
    assert_eq!(steps[0].label(), "RemoteDesktop.type_text");
    assert_eq!(world.count("keymap"), 0);
    assert_eq!(world.count("plan_text_keysyms"), 1);
    assert_eq!(world.count("sent"), 2);
}

#[test]
fn an_unreadable_frame_on_wayland_still_types_and_an_inactive_one_does_not() {
    let unreadable = FakeWorld::wayland().active_script(&[None]);
    type_text_steps(
        &unreadable,
        &element(),
        "hi",
        InteractionPolicy::focus_fallback(),
        deadline(),
    )
    .unwrap();
    assert_eq!(unreadable.count("sent"), 1);

    let inactive = FakeWorld::wayland().active_script(&[Some(false)]);
    let error = type_text_steps(
        &inactive,
        &element(),
        "hi",
        InteractionPolicy::focus_fallback(),
        deadline(),
    )
    .unwrap_err();
    assert_eq!(check_of(&error).as_deref(), Some("window_active"));
    assert!(inactive.sent_nothing());
}

#[test]
fn an_unreadable_frame_on_x11_is_not_taken_as_active() {
    let world = FakeWorld::x11().active_script(&[None]);
    let error = type_text_steps(
        &world,
        &element(),
        "hi",
        InteractionPolicy::headed(),
        deadline(),
    )
    .unwrap_err();
    assert_eq!(check_of(&error).as_deref(), Some("window_active"));
    assert!(world.sent_nothing());
}
