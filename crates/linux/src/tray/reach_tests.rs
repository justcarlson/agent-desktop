use super::*;
use crate::tree::Extents;

const DISPLAY: Extents = Extents {
    x: 0,
    y: 0,
    width: 1920,
    height: 1080,
};

fn item() -> Option<Rect> {
    Some(Rect {
        x: 1800.0,
        y: 0.0,
        width: 30.0,
        height: 30.0,
    })
}

#[test]
fn a_point_inside_the_item_reaches_it() {
    let point = Point { x: 1810.0, y: 10.0 };
    assert_eq!(
        reach(item(), &point, Some(&[DISPLAY])),
        HitTestResult::ReachesTarget
    );
}

#[test]
fn a_point_outside_the_item_is_unknown() {
    let point = Point { x: 100.0, y: 500.0 };
    assert_eq!(
        reach(item(), &point, Some(&[DISPLAY])),
        HitTestResult::Unknown
    );
}

#[test]
fn an_item_with_no_extents_is_unknown() {
    let point = Point { x: 1810.0, y: 10.0 };
    assert_eq!(
        reach(None, &point, Some(&[DISPLAY])),
        HitTestResult::Unknown
    );
}
