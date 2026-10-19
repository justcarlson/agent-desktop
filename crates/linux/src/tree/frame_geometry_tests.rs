use super::*;

fn rect(x: i32, y: i32, width: i32, height: i32) -> Extents {
    Extents {
        x,
        y,
        width,
        height,
    }
}

fn toolkit(name: &str, version: &str) -> ToolkitId {
    ToolkitId {
        name: Some(name.to_owned()),
        version: Some(version.to_owned()),
    }
}

#[test]
fn a_global_toolkit_keeps_its_screen_extents() {
    let screen = rect(100, 137, 420, 34);
    let bounds = pointer_bounds(&toolkit("gtk", "3.24"), screen, rect(0, 0, 420, 34), None);
    assert_eq!(bounds, Some(screen));
}

#[test]
fn gtk_four_position_is_the_window_extents_plus_the_client_origin() {
    let bounds = pointer_bounds(
        &toolkit("gtk", "4.14"),
        rect(0, 0, 558, 54),
        rect(329, 285, 558, 54),
        Some((302, 260)),
    );
    assert_eq!(bounds, Some(rect(631, 545, 558, 54)));
}

#[test]
fn gtk_four_without_a_client_origin_has_no_position_and_never_the_screen_extents() {
    let bounds = pointer_bounds(
        &toolkit("gtk", "4.14"),
        rect(0, 0, 558, 54),
        rect(329, 285, 558, 54),
        None,
    );
    assert_eq!(bounds, None);
}

#[test]
fn an_unreadable_toolkit_is_positioned_like_gtk_four() {
    let unknown = ToolkitId::default();
    let bounds = pointer_bounds(&unknown, rect(0, 0, 9, 9), rect(5, 6, 9, 9), Some((10, 20)));
    assert_eq!(bounds, Some(rect(15, 26, 9, 9)));
}

#[test]
fn a_sentinel_window_position_has_no_pointer_bounds() {
    let bounds = pointer_bounds(
        &toolkit("gtk", "4.14"),
        rect(0, 0, 9, 9),
        rect(i32::MIN, i32::MIN, 9, 9),
        Some((1, 1)),
    );
    assert_eq!(bounds, None);
}

#[test]
fn origins_agree_within_one_pixel_and_not_two() {
    let screen = rect(100, 137, 420, 34);
    let window = rect(0, 0, 420, 34);
    assert!(origins_agree(screen, window, (100, 137)));
    assert!(origins_agree(screen, window, (101, 136)));
    assert!(!origins_agree(screen, window, (102, 137)));
    assert!(!origins_agree(screen, window, (100, 139)));
}

#[test]
fn a_window_offset_is_part_of_the_agreement() {
    let screen = rect(117, 142, 420, 34);
    assert!(origins_agree(screen, rect(17, 5, 420, 34), (100, 137)));
    assert!(!origins_agree(screen, rect(0, 0, 420, 34), (100, 137)));
}

#[test]
fn a_global_toolkit_whose_origin_disagrees_is_not_calibrated() {
    let gtk3 = toolkit("gtk", "3.24");
    let screen = rect(100, 137, 420, 34);
    assert!(calibrate(&gtk3, screen, rect(0, 0, 420, 34), Some((100, 137))).is_some());
    assert_eq!(
        calibrate(&gtk3, screen, rect(0, 0, 420, 34), Some((300, 137))),
        None
    );
    assert_eq!(calibrate(&gtk3, screen, rect(0, 0, 420, 34), None), None);
}

#[test]
fn the_window_point_subtracts_the_window_origin() {
    let gtk3 = toolkit("gtk", "3.24");
    let calibrated = calibrate(
        &gtk3,
        rect(100, 137, 420, 34),
        rect(0, 0, 420, 34),
        Some((100, 137)),
    )
    .expect("agreeing origins calibrate");
    assert_eq!(
        window_point(calibrated.window_origin, (310, 154)),
        Some((210, 17))
    );
}

#[test]
fn a_window_offset_shifts_the_window_origin_of_a_global_toolkit() {
    let gtk3 = toolkit("gtk", "3.24");
    let calibrated = calibrate(
        &gtk3,
        rect(117, 142, 420, 34),
        rect(17, 5, 420, 34),
        Some((100, 137)),
    )
    .expect("agreeing origins calibrate");
    assert_eq!(calibrated.window_origin, (100, 137));
    assert_eq!(
        window_point(calibrated.window_origin, (310, 154)),
        Some((210, 17))
    );
}

#[test]
fn gtk_four_calibrates_to_the_client_origin_and_ignores_the_screen_extents() {
    let calibrated = calibrate(
        &toolkit("gtk", "4.14"),
        rect(0, 0, 558, 54),
        rect(329, 285, 558, 54),
        Some((302, 260)),
    )
    .expect("a client origin calibrates gtk 4");
    assert_eq!(calibrated.rect, rect(631, 545, 558, 54));
    assert_eq!(calibrated.window_origin, (302, 260));
    assert_eq!(
        window_point(calibrated.window_origin, (700, 560)),
        Some((398, 300))
    );
}
