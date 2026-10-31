use std::time::Duration;

use super::*;

#[test]
fn a_frame_presented_at_or_after_the_request_is_fresh() {
    let requested = Duration::new(6, 123_132_548);
    assert!(is_fresh(requested, Some(Duration::new(6, 169_297_839))));
    assert!(is_fresh(requested, Some(requested)));
}

#[test]
fn a_frame_presented_before_the_request_or_without_a_time_is_stale() {
    let requested = Duration::new(6, 123_132_548);
    assert!(!is_fresh(requested, Some(Duration::new(6, 123_132_547))));
    assert!(!is_fresh(requested, None));
}

#[test]
fn the_monotonic_clock_moves_forward() {
    let first = monotonic_now();
    let second = monotonic_now();
    assert!(second >= first);
    assert!(first > Duration::ZERO);
}
