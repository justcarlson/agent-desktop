use super::*;
use crate::tree::Extents;

fn rect(x: i16, y: i16, width: u16, height: u16) -> GrabRect {
    GrabRect {
        x,
        y,
        width,
        height,
    }
}

fn rows(strips: &[GrabRect]) -> u32 {
    strips.iter().map(|strip| u32::from(strip.height)).sum()
}

#[test]
fn a_4k_image_splits_under_the_limit_and_the_rows_sum_to_the_height() {
    let limit = 262_140;
    let row_bytes = 3840 * 4;
    let strips = rect(0, 0, 3840, 2160).strips(row_bytes, limit).unwrap();
    assert!(strips.len() > 1);
    assert!(
        strips
            .iter()
            .all(|s| usize::from(s.height) * row_bytes <= limit)
    );
    assert_eq!(rows(&strips), 2160);
    assert_eq!(strips.first().unwrap().y, 0);
    let last = strips.last().unwrap();
    assert_eq!(last.y as u32 + u32::from(last.height), 2160);
    for pair in strips.windows(2) {
        assert_eq!(pair[0].y + pair[0].height as i16, pair[1].y);
    }
}

#[test]
fn an_image_smaller_than_one_strip_is_one_request() {
    let strips = rect(5, 7, 100, 50).strips(400, 1_000_000).unwrap();
    assert_eq!(strips, vec![rect(5, 7, 100, 50)]);
}

#[test]
fn a_limit_one_row_too_small_is_rejected_not_looped() {
    assert_eq!(
        rect(0, 0, 100, 50).strips(400, 399),
        Err(GrabError::Resource)
    );
    assert!(rect(0, 0, 100, 50).strips(400, 400).is_ok());
}

#[test]
fn strips_keep_the_origin_column_and_offset_the_rows_by_it() {
    let strips = rect(30, 100, 10, 10).strips(40, 120).unwrap();
    assert_eq!(rows(&strips), 10);
    assert!(strips.iter().all(|s| s.x == 30 && s.width == 10));
    assert_eq!(strips[0].y, 100);
    assert_eq!(strips[1].y, 103);
}

#[test]
fn a_strip_that_overflows_the_wire_coordinate_is_refused() {
    assert_eq!(
        rect(0, i16::MAX, 10, 10).strips(40, 120),
        Err(GrabError::Resource)
    );
}

#[test]
fn clipping_keeps_the_part_on_the_screen() {
    assert_eq!(
        rect(10, 10, 20, 20).clip_to(100, 100),
        Some(rect(10, 10, 20, 20))
    );
    assert_eq!(
        rect(-5, -5, 20, 20).clip_to(100, 100),
        Some(rect(0, 0, 15, 15))
    );
    assert_eq!(
        rect(90, 90, 20, 20).clip_to(100, 100),
        Some(rect(90, 90, 10, 10))
    );
}

#[test]
fn a_rectangle_wholly_off_the_screen_clips_to_nothing() {
    assert_eq!(rect(100, 0, 10, 10).clip_to(100, 100), None);
    assert_eq!(rect(-10, 0, 10, 10).clip_to(100, 100), None);
    assert_eq!(rect(0, 200, 10, 10).clip_to(100, 100), None);
}

#[test]
fn extents_convert_only_when_they_fit_the_wire_and_have_area() {
    let extents = |x, y, width, height| Extents {
        x,
        y,
        width,
        height,
    };
    assert_eq!(
        GrabRect::from_extents(extents(-3, 4, 50, 60)),
        Some(rect(-3, 4, 50, 60))
    );
    assert_eq!(GrabRect::from_extents(extents(0, 0, 0, 60)), None);
    assert_eq!(GrabRect::from_extents(extents(0, 0, 60, -1)), None);
    assert_eq!(GrabRect::from_extents(extents(40_000, 0, 5, 5)), None);
    assert_eq!(GrabRect::from_extents(extents(0, 0, 70_000, 5)), None);
}
