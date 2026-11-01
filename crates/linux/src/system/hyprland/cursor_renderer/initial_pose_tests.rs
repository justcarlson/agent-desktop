use agent_desktop_core::Point;

use super::initial_point;

#[test]
fn the_focused_monitor_middle_in_global_logical_pixels() {
    let reply = br#"[
        {"name": "DP-1", "x": 0, "y": 0, "width": 1920, "height": 1080, "scale": 1.0, "transform": 0, "focused": false},
        {"name": "DP-2", "x": 1920, "y": 0, "width": 2560, "height": 1440, "scale": 2.0, "transform": 0, "focused": true}
    ]"#;
    assert_eq!(
        initial_point(reply),
        Point {
            x: 2560.0,
            y: 360.0
        }
    );
}

#[test]
fn a_focused_monitor_at_a_negative_origin_gives_a_point_on_it() {
    let reply = br#"[
        {"name": "HEADLESS-1", "x": 0, "y": 0, "width": 1280, "height": 800, "scale": 1.0, "transform": 0, "focused": false},
        {"name": "AD-E2E-1", "x": -1024, "y": -640, "width": 1280, "height": 800, "scale": 1.25, "transform": 0, "focused": true}
    ]"#;
    assert_eq!(
        initial_point(reply),
        Point {
            x: -512.0,
            y: -320.0
        }
    );
}

#[test]
fn a_focused_monitor_at_a_positive_origin_below_another_gives_a_point_on_it() {
    let reply = br#"[
        {"name": "DP-1", "x": 0, "y": 0, "width": 1920, "height": 1080, "scale": 1.0, "transform": 0, "focused": false},
        {"name": "DP-3", "x": 300, "y": 1080, "width": 1600, "height": 900, "scale": 1.0, "transform": 0, "focused": true}
    ]"#;
    assert_eq!(
        initial_point(reply),
        Point {
            x: 1100.0,
            y: 1530.0
        }
    );
}

#[test]
fn a_rotated_monitor_swaps_its_sides() {
    let reply =
        br#"[{"width": 1920, "height": 1080, "scale": 1.0, "transform": 1, "focused": true}]"#;
    assert_eq!(initial_point(reply), Point { x: 540.0, y: 960.0 });
}

#[test]
fn with_no_focused_monitor_the_first_is_used() {
    let reply =
        br#"[{"width": 800, "height": 600, "scale": 1.0, "transform": 0, "focused": false}]"#;
    assert_eq!(initial_point(reply), Point { x: 400.0, y: 300.0 });
}

#[test]
fn an_unusable_reply_gives_the_origin() {
    for reply in [
        &b"not json"[..],
        b"[]",
        br#"[{"width": 0, "height": 0, "scale": 0}]"#,
    ] {
        assert_eq!(initial_point(reply), Point { x: 0.0, y: 0.0 });
    }
}
