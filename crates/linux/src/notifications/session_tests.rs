use super::*;
use crate::notifications::fake::{FakeWorld, lease, log, reading, spec};
use agent_desktop_core::{ErrorCode, InteractionPolicy};

fn world(log: &crate::notifications::fake::Log) -> FakeWorld {
    FakeWorld::new(log, vec![reading(&[spec("Mail", "a")], false)])
}

#[test]
fn a_headless_policy_is_refused_before_any_event() {
    let log = log();
    let world = world(&log);
    let lease = lease(500);
    let error = Session::open(&world, InteractionPolicy::headless(), &lease)
        .err()
        .expect("refused");
    assert_eq!(error.code, ErrorCode::PolicyDenied);
    assert!(log.borrow().is_empty());
}

#[test]
fn a_focus_only_policy_is_refused_too() {
    let log = log();
    let world = world(&log);
    let lease = lease(500);
    assert!(Session::open(&world, InteractionPolicy::focus_fallback(), &lease).is_err());
    assert!(log.borrow().is_empty());
}

#[test]
fn a_list_the_session_opened_is_closed_on_success() {
    let log = log();
    let world = world(&log);
    let lease = lease(500);
    let session = Session::open(&world, InteractionPolicy::headed(), &lease).expect("open");
    assert_eq!(session.finish(Ok(1)).expect("ok"), 1);
    assert!(!world.showing.get());
}

#[test]
fn a_list_the_session_opened_is_closed_on_error() {
    let log = log();
    let world = world(&log);
    let lease = lease(500);
    let session = Session::open(&world, InteractionPolicy::headed(), &lease).expect("open");
    let error = session
        .finish::<()>(Err(AdapterError::timeout("late")))
        .expect_err("error");
    assert!(!world.showing.get());
    assert_eq!(error.details.expect("details")["list_restore"], "closed");
}

#[test]
fn a_list_that_was_showing_is_adopted_and_left_open() {
    let log = log();
    let world = world(&log).adopted();
    let lease = lease(500);
    let session = Session::open(&world, InteractionPolicy::headed(), &lease).expect("open");
    session.finish(Ok(())).expect("ok");
    assert!(world.showing.get());
    assert!(!log.borrow().iter().any(|line| line == "close"));
}

#[test]
fn a_close_that_does_not_stick_turns_a_success_into_an_error_naming_it() {
    let log = log();
    let mut world = world(&log);
    world.close_sticks = true;
    let lease = lease(500);
    let session = Session::open(&world, InteractionPolicy::headed(), &lease).expect("open");
    let error = session.finish(Ok(())).expect_err("unverified");
    assert_eq!(
        error.details.expect("details")["list_restore"],
        "unverified"
    );
}

#[test]
fn dropping_an_unfinished_session_closes_what_it_opened() {
    let log = log();
    let world = world(&log);
    let lease = lease(500);
    drop(Session::open(&world, InteractionPolicy::headed(), &lease).expect("open"));
    assert!(!world.showing.get());
}

#[test]
fn an_open_that_failed_after_showing_the_list_still_closes_it() {
    let log = log();
    let mut world = world(&log);
    world.open_fails = true;
    let lease = lease(500);
    let error = Session::open(&world, InteractionPolicy::headed(), &lease)
        .err()
        .expect("error");
    assert!(!world.showing.get());
    assert_eq!(error.details.expect("details")["list_restore"], "closed");
}

#[test]
fn the_list_is_closed_even_when_the_command_spent_its_whole_budget() {
    let log = log();
    let world = world(&log);
    let lease = lease(5);
    let session = Session::open(&world, InteractionPolicy::headed(), &lease).expect("open");
    std::thread::sleep(std::time::Duration::from_millis(30));
    assert_eq!(session.finish(Ok(1)).expect("closed"), 1);
    assert!(!world.showing.get());
}
