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
fn the_clear_all_button_is_found_without_its_english_name() {
    let log = log();
    let mut full = reading(&mixed(), true);
    for node in full
        .nodes
        .iter_mut()
        .filter(|node| node.name.starts_with("Clear"))
    {
        node.name = "Alle Benachrichtigungen l\u{f6}schen".to_owned();
    }
    let world = FakeWorld::new(&log, vec![full, reading(&[], true)]);
    let (dismissed, _) = dismiss_all_in(
        &world,
        request(None, InteractionPolicy::headed()),
        &lease(600),
    )
    .expect("cleared");
    assert_eq!(dismissed.len(), 3);
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
    assert_eq!(clicks, ["/e1/close", "/e2/close"]);
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
fn a_filtered_click_error_is_a_per_entry_failure_not_an_abort() {
    let log = log();
    let mut world = FakeWorld::new(&log, vec![reading(&mixed(), true)]);
    world.faults.click = true;
    let (dismissed, failures) = dismiss_all_in(
        &world,
        request(Some("mail"), InteractionPolicy::headed()),
        &lease(600),
    )
    .expect("outcome");
    assert!(dismissed.is_empty());
    assert_eq!(failures.len(), 2);
}

#[test]
fn an_empty_list_is_cleared_without_opening_it() {
    let log = log();
    let world = FakeWorld::new(&log, vec![reading(&[], true)]);
    let (dismissed, failures) = dismiss_all_in(
        &world,
        request(None, InteractionPolicy::headed()),
        &lease(600),
    )
    .expect("cleared");
    assert!(dismissed.is_empty() && failures.is_empty());
    assert!(world.clicks().is_empty());
    assert!(!log.borrow().iter().any(|line| line == "open"));
}

#[test]
fn a_list_that_cannot_be_read_after_the_clear_is_a_delivered_unverified_failure() {
    let log = log();
    let mut world = FakeWorld::new(&log, vec![reading(&mixed(), true)]);
    world.faults.read_after_click = true;
    let error = dismiss_all_in(
        &world,
        request(None, InteractionPolicy::headed()),
        &lease(600),
    )
    .expect_err("unconfirmed");
    assert_eq!(
        error.disposition,
        agent_desktop_core::DeliverySemantics::delivered_unverified()
    );
}
