use super::*;
use crate::notifications::fake::{FakeWorld, log, reading, spec};
use agent_desktop_core::{Deadline, ErrorCode};

fn deadline() -> Deadline {
    Deadline::after(1_000).expect("a deadline")
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
