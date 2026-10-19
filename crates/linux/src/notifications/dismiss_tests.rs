use super::*;
use crate::notifications::fake::{FakeWorld, Spec, lease, log, reading, spec};
use agent_desktop_core::{ErrorCode, InteractionPolicy, NotificationIdentity};

fn three() -> Vec<Spec> {
    vec![spec("Mail", "a"), spec("Chat", "b"), spec("Files", "c")]
}

fn identity(app: &str, title: &str) -> NotificationIdentity {
    NotificationIdentity {
        expected_app: Some(app.to_owned()),
        expected_title: Some(title.to_owned()),
    }
}

fn request<'a>(
    index: usize,
    identity: &'a NotificationIdentity,
    policy: InteractionPolicy,
) -> DismissNotificationRequest<'a> {
    DismissNotificationRequest {
        index,
        app_filter: None,
        identity,
        policy,
    }
}

fn run(
    world: FakeWorld,
    identity: &NotificationIdentity,
) -> Result<NotificationInfo, AdapterError> {
    dismiss_in(
        &world,
        request(2, identity, InteractionPolicy::headed()),
        &lease(600),
    )
}

fn world_clicks(log: &crate::notifications::fake::Log) -> Vec<String> {
    log.borrow()
        .iter()
        .filter_map(|line| line.strip_prefix("click:").map(str::to_owned))
        .collect()
}

#[test]
fn the_close_button_of_the_target_is_clicked_and_nothing_else() {
    let log = log();
    let stages = vec![
        reading(&three(), false),
        reading(&[spec("Mail", "a"), spec("Files", "c")], false),
    ];
    let world = FakeWorld::new(&log, stages);
    let done = run(world, &identity("Chat", "b")).expect("dismissed");
    assert_eq!(done.title, "b");
    assert_eq!(world_clicks(&log), vec!["/e2/close"]);
}

#[test]
fn a_mismatched_identity_clicks_nothing() {
    let log = log();
    let world = FakeWorld::new(&log, vec![reading(&three(), false)]);
    let error = run(world, &identity("Mail", "a")).expect_err("moved");
    assert_eq!(error.code, ErrorCode::NotificationNotFound);
    assert!(!log.borrow().iter().any(|line| line.starts_with("click")));
}

#[test]
fn a_reflow_between_the_reads_is_refused_before_the_click() {
    let log = log();
    let moved = vec![spec("New", "n"), spec("Mail", "a"), spec("Chat", "b")];
    let world = FakeWorld::new(&log, vec![reading(&three(), false), reading(&moved, false)])
        .staged_by_reads();
    let error = run(world, &identity("Chat", "b")).expect_err("moved");
    assert_eq!(error.code, ErrorCode::NotificationNotFound);
    assert!(!log.borrow().iter().any(|line| line.starts_with("click")));
}

#[test]
fn a_different_entry_leaving_with_the_target_fails_the_call() {
    let log = log();
    let stages = vec![
        reading(&three(), false),
        reading(&[spec("Mail", "a")], false),
    ];
    let world = FakeWorld::new(&log, stages);
    let error = run(world, &identity("Chat", "b")).expect_err("bystander");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert!(error.message.contains("other than the target"));
}

#[test]
fn a_different_entry_leaving_instead_of_the_target_fails_the_call() {
    let log = log();
    let stages = vec![
        reading(&three(), false),
        reading(&[spec("Chat", "b"), spec("Files", "c")], false),
    ];
    let world = FakeWorld::new(&log, stages);
    let error = run(world, &identity("Chat", "b")).expect_err("bystander");
    assert!(error.message.contains("other than the target"));
}

#[test]
fn two_identical_entries_pass_only_when_exactly_one_remains() {
    let twins = || vec![spec("Mail", "x"), spec("Mail", "x")];
    let log_one = log();
    let one = FakeWorld::new(
        &log_one,
        vec![
            reading(&twins(), false),
            reading(&[spec("Mail", "x")], false),
        ],
    );
    assert!(run(one, &identity("Mail", "x")).is_ok());
    let log_none = log();
    let none = FakeWorld::new(
        &log_none,
        vec![reading(&twins(), false), reading(&[], false)],
    );
    assert!(run(none, &identity("Mail", "x")).is_err());
}

#[test]
fn a_click_the_shell_ignored_is_an_unverified_failure() {
    let log = log();
    let world = FakeWorld::new(&log, vec![reading(&three(), false)]);
    let error = run(world, &identity("Chat", "b")).expect_err("survived");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(
        error.disposition,
        agent_desktop_core::DeliverySemantics::delivered_unverified()
    );
}

#[test]
fn a_headless_policy_is_refused_with_no_event() {
    let log = log();
    let world = FakeWorld::new(&log, vec![reading(&three(), false)]);
    let none = NotificationIdentity::default();
    let error = dismiss_in(
        &world,
        request(2, &none, InteractionPolicy::headless()),
        &lease(600),
    )
    .expect_err("denied");
    assert_eq!(error.code, ErrorCode::PolicyDenied);
    assert!(log.borrow().is_empty());
}

#[test]
fn the_list_a_failed_dismissal_opened_is_closed() {
    let log = log();
    let world = FakeWorld::new(&log, vec![reading(&three(), false)]);
    let _ = run(world, &identity("Mail", "a"));
    assert!(log.borrow().iter().any(|line| line == "close"));
}

#[test]
fn an_adopted_list_stays_open_after_a_dismissal() {
    let log = log();
    let stages = vec![
        reading(&three(), false),
        reading(&[spec("Mail", "a"), spec("Files", "c")], false),
    ];
    let world = FakeWorld::new(&log, stages).adopted();
    run(world, &identity("Chat", "b")).expect("dismissed");
    assert!(!log.borrow().iter().any(|line| line == "close"));
}

#[test]
fn a_click_error_is_returned_and_the_list_is_still_restored() {
    let log = log();
    let mut world = FakeWorld::new(&log, vec![reading(&three(), false)]);
    world.click_fails = true;
    assert!(run(world, &identity("Chat", "b")).is_err());
    assert!(log.borrow().iter().any(|line| line == "close"));
}
