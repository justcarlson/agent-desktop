use super::*;

fn size(width: i32, height: i32) -> Extents {
    Extents {
        x: 0,
        y: 0,
        width,
        height,
    }
}

fn bounds(width: f64, height: f64) -> Option<Rect> {
    Some(Rect {
        x: 0.0,
        y: 0.0,
        width,
        height,
    })
}

#[test]
fn the_frame_wins_when_it_matches_the_bounds_and_the_client_does_not() {
    let unit = choose(
        size(1000, 700),
        Some(size(1002, 739)),
        bounds(1000.0, 737.0),
    );
    assert_eq!(unit, Unit::Frame);
}

#[test]
fn the_client_is_taken_when_the_frame_is_off_but_the_client_matches() {
    let unit = choose(
        size(1000, 737),
        Some(size(1040, 800)),
        bounds(1000.0, 737.0),
    );
    assert_eq!(unit, Unit::Client);
}

#[test]
fn a_frame_two_percent_off_fails_the_strict_check_but_the_client_check_is_looser() {
    let frame = size(1019, 737);
    assert_eq!(
        choose(size(500, 500), Some(frame), bounds(1000.0, 737.0)),
        Unit::Unproven
    );
    assert_eq!(choose(frame, None, bounds(1000.0, 737.0)), Unit::Client);
}

#[test]
fn neither_size_matching_is_unproven() {
    let unit = choose(
        size(1000, 700),
        Some(size(1002, 760)),
        bounds(1000.0, 737.0),
    );
    assert_eq!(unit, Unit::Unproven);
}

#[test]
fn missing_or_empty_bounds_prove_nothing() {
    assert_eq!(
        choose(size(10, 10), Some(size(10, 10)), None),
        Unit::Unproven
    );
    assert_eq!(choose(size(10, 10), None, bounds(0.0, 0.0)), Unit::Unproven);
}

#[test]
fn a_plain_screenshot_falls_back_to_the_frame_then_the_client() {
    assert_eq!(for_screenshot(Unit::Unproven, true), Unit::Frame);
    assert_eq!(for_screenshot(Unit::Unproven, false), Unit::Client);
    assert_eq!(for_screenshot(Unit::Client, true), Unit::Client);
}
