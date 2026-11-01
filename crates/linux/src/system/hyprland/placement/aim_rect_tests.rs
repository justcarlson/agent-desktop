use agent_desktop_core::Rect;

use super::aim_rect;
use crate::tree::Extents;

fn extents(x: i32, y: i32, width: i32, height: i32) -> Extents {
    Extents {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn an_unclipped_element_keeps_its_rectangle() {
    assert_eq!(
        aim_rect(Some(extents(10, 20, 30, 40)), None),
        Some(Rect {
            x: 10.0,
            y: 20.0,
            width: 30.0,
            height: 40.0
        })
    );
}

#[test]
fn a_clipped_row_is_aimed_only_inside_its_viewport() {
    assert_eq!(
        aim_rect(Some(extents(0, 90, 100, 40)), Some(extents(0, 0, 200, 100))),
        Some(Rect {
            x: 0.0,
            y: 90.0,
            width: 100.0,
            height: 10.0
        })
    );
}

#[test]
fn a_row_scrolled_out_of_view_or_without_area_has_no_aim() {
    assert_eq!(
        aim_rect(
            Some(extents(0, 150, 100, 40)),
            Some(extents(0, 0, 200, 100))
        ),
        None
    );
    assert_eq!(aim_rect(Some(extents(0, 0, 0, 10)), None), None);
    assert_eq!(aim_rect(None, None), None);
}
