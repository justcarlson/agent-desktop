use super::*;

const FRAME: Extents = Extents {
    x: 0,
    y: 0,
    width: 420,
    height: 420,
};

fn rect(x: i32, y: i32, width: i32, height: i32) -> Extents {
    Extents {
        x,
        y,
        width,
        height,
    }
}

fn input(node: Option<Extents>) -> OffscreenInput {
    OffscreenInput {
        node,
        frame: Some(FRAME),
        container: None,
        visible: true,
        showing: true,
        sentinel: false,
        minimized: Some(false),
    }
}

#[test]
fn a_gtk3_row_scrolled_out_is_visible_without_showing_at_the_sentinel() {
    let mut row = input(None);
    row.showing = false;
    row.sentinel = true;
    row.frame = None;
    assert_eq!(offscreen(row), Some(true));
}

#[test]
fn the_sentinel_alone_is_not_the_gtk3_shape() {
    let mut row = input(None);
    row.sentinel = true;
    assert_eq!(offscreen(row), None);
    row.visible = false;
    row.showing = false;
    assert_eq!(offscreen(row), None);
}

#[test]
fn showing_absent_on_a_node_inside_the_frame_is_not_offscreen() {
    let mut web = input(Some(rect(40, 112, 304, 35)));
    web.showing = false;
    assert_eq!(offscreen(web), Some(false));
    web.minimized = None;
    assert_eq!(offscreen(web), None);
}

#[test]
fn a_row_far_below_the_frame_is_offscreen() {
    assert_eq!(offscreen(input(Some(rect(2, 5526, 420, 22)))), Some(true));
}

#[test]
fn a_node_straddling_the_frame_edge_is_on_screen() {
    assert_eq!(offscreen(input(Some(rect(2, 410, 100, 40)))), Some(false));
}

#[test]
fn a_node_just_outside_the_frame_inside_the_slack_proves_nothing() {
    assert_eq!(offscreen(input(Some(rect(0, 430, 100, 20)))), None);
    let past = EDGE_SLACK + 2;
    assert_eq!(
        offscreen(input(Some(rect(0, 420 + past, 100, 20)))),
        Some(true)
    );
}

#[test]
fn no_frame_or_no_positive_area_is_unknown() {
    let mut unframed = input(Some(rect(0, 5000, 10, 10)));
    unframed.frame = None;
    assert_eq!(offscreen(unframed), None);
    assert_eq!(offscreen(input(Some(rect(0, 5000, 710, 0)))), None);
    assert_eq!(offscreen(input(None)), None);
    assert_eq!(offscreen(input(Some(rect(i32::MIN, i32::MIN, 5, 5)))), None);
}

#[test]
fn a_node_wholly_outside_the_container_but_inside_the_frame_is_offscreen() {
    let mut row = input(Some(rect(0, 300, 100, 20)));
    row.container = Some(rect(0, 0, 420, 100));
    assert_eq!(offscreen(row), Some(true));
    row.node = Some(rect(0, 50, 100, 20));
    assert_eq!(offscreen(row), Some(false));
    row.node = Some(rect(0, 110, 100, 20));
    assert_eq!(offscreen(row), None);
}

#[test]
fn a_container_of_no_area_cannot_prove_the_node_on_screen() {
    let mut row = input(Some(rect(0, 50, 100, 20)));
    row.container = Some(rect(0, 0, 420, 0));
    assert_eq!(offscreen(row), None);
}

#[test]
fn a_minimized_window_hides_every_node_whatever_its_rectangle() {
    let mut row = input(Some(rect(0, 50, 100, 20)));
    row.minimized = Some(true);
    assert_eq!(offscreen(row), Some(true));
    row.node = None;
    row.frame = None;
    assert_eq!(offscreen(row), Some(true));
}

#[test]
fn a_container_at_the_sentinel_position_is_ignored_and_the_frame_decides() {
    let mut row = input(Some(rect(0, 0, 480, 34)));
    row.container = Some(rect(i32::MIN, i32::MIN, 480, 360));
    assert_eq!(offscreen(row), Some(false));
    row.node = Some(rect(2, 5526, 420, 22));
    assert_eq!(offscreen(row), Some(true));
}
