use std::time::Instant;

use super::{WINDOW, mark, settles, settles_at};

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
