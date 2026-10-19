use super::*;
use crate::notifications::fake::{FakeWorld, log, reading, spec};
use agent_desktop_core::{Deadline, ErrorCode};

fn deadline() -> Deadline {
    Deadline::after(1_000).expect("a deadline")
}

fn identity(app: &str, title: &str) -> NotificationIdentity {
    NotificationIdentity {
        expected_app: Some(app.to_owned()),
        expected_title: Some(title.to_owned()),
    }
}

#[test]
fn the_entry_at_the_index_is_picked() {
    let world = FakeWorld::new(
        &log(),
        vec![reading(&[spec("A", "one"), spec("B", "two")], false)],
    );
    let found = locate(&world, 2, None, &identity("B", "two"), deadline()).expect("found");
    assert_eq!(found.entry.info.title, "two");
    assert_eq!(found.all.len(), 2);
}

#[test]
fn an_empty_identity_accepts_the_entry() {
    let world = FakeWorld::new(&log(), vec![reading(&[spec("A", "one")], false)]);
    assert!(
        locate(
            &world,
            1,
            None,
            &NotificationIdentity::default(),
            deadline()
        )
        .is_ok()
    );
}

#[test]
fn a_different_identity_at_the_index_is_not_found() {
    let world = FakeWorld::new(
        &log(),
        vec![reading(&[spec("A", "one"), spec("B", "two")], false)],
    );
    let error = locate(&world, 1, None, &identity("B", "two"), deadline())
        .err()
        .expect("moved");
    assert_eq!(error.code, ErrorCode::NotificationNotFound);
}

#[test]
fn an_index_past_the_end_is_not_found() {
    let world = FakeWorld::new(&log(), vec![reading(&[spec("A", "one")], false)]);
    let error = locate(
        &world,
        5,
        None,
        &NotificationIdentity::default(),
        deadline(),
    )
    .err()
    .expect("none");
    assert_eq!(error.code, ErrorCode::NotificationNotFound);
}

#[test]
fn an_app_filter_that_does_not_match_the_entry_is_not_found() {
    let world = FakeWorld::new(&log(), vec![reading(&[spec("A", "one")], false)]);
    let error = locate(
        &world,
        1,
        Some("zzz"),
        &NotificationIdentity::default(),
        deadline(),
    )
    .err()
    .expect("none");
    assert_eq!(error.code, ErrorCode::NotificationNotFound);
}

#[test]
fn reconfirm_refuses_an_entry_that_changed_between_reads() {
    let before = reading(&[spec("A", "one"), spec("B", "two")], false);
    let after = reading(&[spec("N", "new"), spec("A", "one")], false);
    let world = FakeWorld::new(&log(), vec![before, after]).staged_by_reads();
    let none = NotificationIdentity::default();
    let first = locate(&world, 2, None, &none, deadline()).expect("first");
    let error = reconfirm(&world, 2, None, &none, &first.entry.info, deadline())
        .err()
        .expect("moved");
    assert_eq!(error.code, ErrorCode::NotificationNotFound);
}
