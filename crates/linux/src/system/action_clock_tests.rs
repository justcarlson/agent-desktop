use std::time::{Duration, Instant};

use super::{WINDOW, mark, settles, settles_at, within};

#[test]
fn a_process_that_delivered_no_action_does_not_wait() {
    assert!(!within(None, Instant::now()));
}

#[test]
fn an_action_just_delivered_makes_the_next_listing_wait() {
    let now = Instant::now();
    assert!(within(Some(now), now + Duration::from_millis(100)));
}

#[test]
fn an_old_action_no_longer_makes_a_listing_wait() {
    let now = Instant::now();
    assert!(!within(Some(now), now + WINDOW));
}

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
fn marking_an_action_makes_it_recent() {
    mark();
    assert!(settles(true));
    assert!(!settles(false));
}
