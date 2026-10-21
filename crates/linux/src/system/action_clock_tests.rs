use std::time::Instant;

use super::{LAST_ACTION, WINDOW, mark_outcome, settles, settles_at};

#[test]
fn only_a_focused_window_with_a_recent_action_waits_for_a_popup() {
    let now = Instant::now();
    let recent_action = Some(now);
    let old_action = Some(now - WINDOW);
    assert!(settles_at(true, recent_action, now));
    assert!(!settles_at(false, recent_action, now));
    assert!(!settles_at(true, old_action, now));
    assert!(!settles_at(true, None, now));
}

#[test]
fn only_started_actions_advance_the_popup_clock() {
    use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode};
    *LAST_ACTION.lock().expect("clock") = None;
    let refused: Result<(), _> = Err(AdapterError::new(ErrorCode::ActionFailed, "refused")
        .with_disposition(DeliverySemantics::not_delivered()));
    mark_outcome(Some(42), &refused);
    assert!(LAST_ACTION.lock().expect("clock").is_none());
    let uncertain: Result<(), _> = Err(AdapterError::new(ErrorCode::ActionFailed, "unconfirmed")
        .with_disposition(DeliverySemantics::uncertain()));
    mark_outcome(Some(42), &uncertain);
    assert!(settles(true));
    assert!(!settles(false));
    let before = *LAST_ACTION.lock().expect("clock");
    mark_outcome(Some(42), &refused);
    assert_eq!(*LAST_ACTION.lock().expect("clock"), before);
    mark_outcome(Some(42), &Ok::<_, AdapterError>(()));
    assert!(LAST_ACTION.lock().expect("clock").is_some());
}
