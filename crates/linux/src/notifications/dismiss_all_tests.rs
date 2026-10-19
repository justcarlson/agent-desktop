use super::*;
use crate::notifications::fake::{FakeWorld, Spec, lease, log, reading, spec};
use agent_desktop_core::InteractionPolicy;

fn mixed() -> Vec<Spec> {
    vec![spec("Mail", "a1"), spec("Chat", "b1"), spec("Mail", "a2")]
}

fn request(app: Option<&str>, policy: InteractionPolicy) -> DismissAllNotificationsRequest<'_> {
    DismissAllNotificationsRequest {
        app_filter: app,
        policy,
    }
}

#[test]
fn without_a_filter_one_click_clears_everything() {
    let log = log();
    let world = FakeWorld::new(&log, vec![reading(&mixed(), true), reading(&[], true)]);
    let (dismissed, failures) = dismiss_all_in(
        &world,
        request(None, InteractionPolicy::headed()),
        &lease(600),
    )
    .expect("cleared");
    assert_eq!(dismissed.len(), 3);
    assert!(failures.is_empty());
    assert_eq!(world.clicks(), vec!["/clear"]);
}

#[test]
fn a_filter_never_clicks_clear_all_and_leaves_other_apps() {
    let log = log();
    let stages = vec![
        reading(&mixed(), true),
        reading(&[spec("Chat", "b1"), spec("Mail", "a2")], true),
        reading(&[spec("Chat", "b1")], true),
    ];
    let world = FakeWorld::new(&log, stages);
    let (dismissed, failures) = dismiss_all_in(
        &world,
        request(Some("mail"), InteractionPolicy::headed()),
        &lease(900),
    )
    .expect("cleared");
    assert_eq!(dismissed.len(), 2);
    assert!(failures.is_empty());
    let clicks = world.clicks();
    assert!(!clicks.iter().any(|path| path == "/clear"));
    assert_eq!(clicks.len(), 2);
}

#[test]
fn a_filtered_dismissal_re_reads_between_clicks() {
    let log = log();
    let stages = vec![
        reading(&mixed(), true),
        reading(&[spec("Chat", "b1"), spec("Mail", "a2")], true),
        reading(&[spec("Chat", "b1")], true),
    ];
    let world = FakeWorld::new(&log, stages);
    dismiss_all_in(
        &world,
        request(Some("mail"), InteractionPolicy::headed()),
        &lease(900),
    )
    .expect("cleared");
    let lines = log.borrow().clone();
    let first_click = lines
        .iter()
        .position(|line| line.starts_with("click"))
        .expect("click");
    let second_click = lines
        .iter()
        .rposition(|line| line.starts_with("click"))
        .expect("click");
    assert!(
        lines[first_click..second_click]
            .iter()
            .any(|line| line == "read")
    );
}

#[test]
fn an_ignored_clear_reports_each_survivor() {
    let log = log();
    let world = FakeWorld::new(&log, vec![reading(&mixed(), true)]);
    let (dismissed, failures) = dismiss_all_in(
        &world,
        request(None, InteractionPolicy::headed()),
        &lease(400),
    )
    .expect("outcome");
    assert!(dismissed.is_empty());
    assert_eq!(failures.len(), 3);
}

#[test]
fn entries_that_arrived_during_the_clear_are_not_failures() {
    let log = log();
    let world = FakeWorld::new(
        &log,
        vec![reading(&mixed(), true), reading(&[spec("New", "n")], true)],
    );
    let (dismissed, failures) = dismiss_all_in(
        &world,
        request(None, InteractionPolicy::headed()),
        &lease(600),
    )
    .expect("outcome");
    assert_eq!(dismissed.len(), 3);
    assert!(failures.is_empty());
}

#[test]
fn an_empty_list_clicks_nothing() {
    let log = log();
    let world = FakeWorld::new(&log, vec![reading(&[], true)]);
    let (dismissed, failures) = dismiss_all_in(
        &world,
        request(None, InteractionPolicy::headed()),
        &lease(600),
    )
    .expect("outcome");
    assert!(dismissed.is_empty() && failures.is_empty());
    assert!(world.clicks().is_empty());
}

#[test]
fn a_missing_clear_all_button_is_an_error() {
    let log = log();
    let world = FakeWorld::new(&log, vec![reading(&mixed(), false)]);
    assert!(
        dismiss_all_in(
            &world,
            request(None, InteractionPolicy::headed()),
            &lease(600)
        )
        .is_err()
    );
}

#[test]
fn headless_is_refused_with_no_event() {
    let log = log();
    let world = FakeWorld::new(&log, vec![reading(&mixed(), true)]);
    assert!(
        dismiss_all_in(
            &world,
            request(None, InteractionPolicy::headless()),
            &lease(600)
        )
        .is_err()
    );
    assert!(log.borrow().is_empty());
}

#[test]
fn the_list_is_closed_after_a_failed_clear() {
    let log = log();
    let mut world = FakeWorld::new(&log, vec![reading(&mixed(), true)]);
    world.click_fails = true;
    assert!(
        dismiss_all_in(
            &world,
            request(None, InteractionPolicy::headed()),
            &lease(600)
        )
        .is_err()
    );
    assert!(log.borrow().iter().any(|line| line == "close"));
}

#[test]
fn a_filtered_click_error_is_a_per_entry_failure_not_an_abort() {
    let log = log();
    let mut world = FakeWorld::new(&log, vec![reading(&mixed(), true)]);
    world.click_fails = true;
    let (dismissed, failures) = dismiss_all_in(
        &world,
        request(Some("mail"), InteractionPolicy::headed()),
        &lease(600),
    )
    .expect("outcome");
    assert!(dismissed.is_empty());
    assert_eq!(failures.len(), 2);
}
