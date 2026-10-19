use agent_desktop_core::{HitTestResult, Point};

use crate::system::shell::fake_tree::{SENTINEL, at};
use crate::system::shell::hit::decide;
use crate::tree::Extents;

fn displays() -> Vec<Extents> {
    vec![at(0, 0, 1280, 800)]
}

fn point(x: f64, y: f64) -> Point {
    Point { x, y }
}

#[test]
fn a_point_inside_an_on_display_node_reaches_it() {
    let verdict = decide(
        Some(at(0, 0, 72, 32)),
        &point(36.0, 16.0),
        Some(&displays()),
    );
    assert_eq!(verdict, HitTestResult::ReachesTarget);
}

#[test]
fn the_sentinel_and_an_off_display_node_are_unknown() {
    for extents in [SENTINEL, at(-84, 549, 84, 60)] {
        let verdict = decide(Some(extents), &point(-40.0, 570.0), Some(&displays()));
        assert_eq!(verdict, HitTestResult::Unknown);
    }
}

#[test]
fn a_point_outside_the_node_is_unknown_never_intercepted() {
    let verdict = decide(
        Some(at(0, 0, 72, 32)),
        &point(500.0, 500.0),
        Some(&displays()),
    );
    assert_eq!(verdict, HitTestResult::Unknown);
}

#[test]
fn nothing_read_is_unknown() {
    assert_eq!(
        decide(None, &point(1.0, 1.0), Some(&displays())),
        HitTestResult::Unknown
    );
    assert_eq!(
        decide(Some(at(0, 0, 10, 10)), &point(1.0, 1.0), None),
        HitTestResult::Unknown
    );
    assert_eq!(
        decide(Some(at(0, 0, 10, 10)), &point(1.0, 1.0), Some(&[])),
        HitTestResult::Unknown
    );
}

#[test]
fn no_input_ever_yields_an_interception() {
    let nodes = [
        Some(at(0, 0, 72, 32)),
        Some(SENTINEL),
        Some(at(-84, 0, 84, 60)),
        None,
    ];
    for node in nodes {
        for x in [-100.0, 0.0, 36.0, 2000.0] {
            let verdict = decide(node, &point(x, 16.0), Some(&displays()));
            assert!(!matches!(verdict, HitTestResult::InterceptedBy { .. }));
        }
    }
}
