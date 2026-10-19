use super::active_of;
use crate::tree::Field;
use crate::tree::test_support::words;

#[test]
fn a_state_set_with_the_active_bit_is_active() {
    assert_eq!(active_of(&Field::Present(words(&[1, 25]))), Some(true));
}

#[test]
fn a_state_set_without_the_active_bit_is_inactive() {
    assert_eq!(active_of(&Field::Present(words(&[8, 25]))), Some(false));
}

#[test]
fn an_unreadable_state_set_is_unknown_and_not_inactive() {
    assert_eq!(active_of(&Field::Failed("gone".to_owned())), None);
    assert_eq!(active_of(&Field::Absent), None);
}

#[test]
fn the_active_bit_is_read_from_its_own_position_only() {
    assert_eq!(active_of(&Field::Present(words(&[33]))), Some(false));
}

use std::time::Duration;

use super::{ACTIVE_BUDGET, ActiveGate, poll_active};
use crate::actions::physical_target::tests::fake::{
    FakeWorld, check_of, deadline, element, not_delivered,
};

#[test]
fn the_poll_stops_at_the_first_active_read() {
    let world = FakeWorld::x11().active_script(&[Some(false), Some(false), Some(true)]);
    let read = poll_active(&world, &element(), ACTIVE_BUDGET, deadline()).unwrap();
    assert_eq!(read, Some(true));
    assert_eq!(world.count("read_active"), 3);
    assert_eq!(world.count("pause"), 2);
}

#[test]
fn a_frame_that_never_activates_is_read_for_the_whole_budget() {
    let world = FakeWorld::x11().active_script(&[Some(false)]);
    let read = poll_active(&world, &element(), ACTIVE_BUDGET, deadline()).unwrap();
    assert_eq!(read, Some(false));
    assert_eq!(world.count("read_active"), 31);
}

#[test]
fn a_zero_budget_reads_once_and_never_pauses() {
    let world = FakeWorld::x11().active_script(&[Some(false)]);
    poll_active(&world, &element(), Duration::ZERO, deadline()).unwrap();
    assert_eq!(world.count("read_active"), 1);
    assert_eq!(world.count("pause"), 0);
}

#[test]
fn the_first_check_of_an_inactive_frame_refuses_before_any_key() {
    let world = FakeWorld::x11().active_script(&[Some(false)]);
    let target = element();
    let error = ActiveGate::new(&world, &target, deadline(), false)
        .check()
        .unwrap_err();
    assert_eq!(check_of(&error).as_deref(), Some("window_active"));
    assert!(not_delivered(&error));
    assert!(error.suggestion.as_deref().unwrap().contains("dialog"));
}

#[test]
fn a_later_check_that_finds_the_frame_inactive_does_not_claim_nothing_was_sent() {
    let world = FakeWorld::x11().active_script(&[Some(true), Some(false)]);
    let target = element();
    let mut gate = ActiveGate::new(&world, &target, deadline(), false);
    gate.check().unwrap();
    let error = gate.check().unwrap_err();
    assert_eq!(check_of(&error).as_deref(), Some("window_active"));
    assert!(!not_delivered(&error));
    assert_eq!(world.count("pause"), 0);
}

#[test]
fn an_unreadable_frame_refuses_on_x11_and_passes_where_it_is_allowed_to() {
    let world = FakeWorld::x11().active_script(&[None]);
    let target = element();
    let strict = ActiveGate::new(&world, &target, deadline(), false).check();
    assert_eq!(
        check_of(&strict.unwrap_err()).as_deref(),
        Some("window_active")
    );
    let lenient = ActiveGate::new(&world, &target, deadline(), true).check();
    assert!(lenient.is_ok());
}
