use agent_desktop_core::{DisplayInfo, ErrorCode, Rect};

use crate::input::shell_aim::aim;

fn display(width: f64, height: f64, scale: f64) -> DisplayInfo {
    DisplayInfo {
        id: "d".to_owned(),
        bounds: Rect {
            x: 0.0,
            y: 0.0,
            width,
            height,
        },
        is_primary: true,
        scale,
    }
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn the_centre_of_activities_is_thirty_six_sixteen() {
    let point = aim(rect(0.0, 0.0, 72.0, 32.0), &[display(1280.0, 800.0, 1.0)])
        .expect("an on-display node aims");
    assert_eq!((point.x, point.y), (36.0, 16.0));
}

#[test]
fn an_autohidden_dock_wholly_off_the_display_is_refused_with_nothing_sent() {
    let error = aim(
        rect(-84.0, 549.0, 84.0, 60.0),
        &[display(1280.0, 800.0, 1.0)],
    )
    .expect_err("x plus width is zero, so it touches no pixel");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(
        error.details.expect("details")["physical_delivery_started"],
        false
    );
}

#[test]
fn a_merely_negative_origin_that_overlaps_a_display_is_accepted() {
    let point = aim(rect(0.0, -1.0, 60.0, 84.0), &[display(1280.0, 800.0, 1.0)])
        .expect("it overlaps the display");
    assert_eq!((point.x, point.y), (30.0, 41.0));
}

#[test]
fn the_invalid_sentinel_and_an_empty_rectangle_are_refused() {
    let displays = [display(1280.0, 800.0, 1.0)];
    let sentinel = f64::from(i32::MIN);
    assert!(aim(rect(sentinel, sentinel, sentinel, sentinel), &displays).is_err());
    assert!(aim(rect(10.0, 10.0, 0.0, 30.0), &displays).is_err());
    assert!(aim(rect(10.0, 10.0, 30.0, 0.0), &displays).is_err());
}

#[test]
fn a_scale_two_display_list_does_not_change_a_screen_coordinate_point() {
    let one = aim(
        rect(1204.0, 0.0, 76.0, 32.0),
        &[display(1280.0, 800.0, 1.0)],
    )
    .expect("aims");
    let two = aim(
        rect(1204.0, 0.0, 76.0, 32.0),
        &[display(1280.0, 800.0, 2.0)],
    )
    .expect("aims");
    assert_eq!(one, two);
}

#[test]
fn any_display_of_several_may_hold_the_node() {
    let mut second = display(1280.0, 800.0, 1.0);
    second.bounds.x = 1280.0;
    let displays = [display(1280.0, 800.0, 1.0), second];
    assert!(aim(rect(1300.0, 10.0, 50.0, 30.0), &displays).is_ok());
    assert!(aim(rect(2600.0, 10.0, 50.0, 30.0), &displays).is_err());
}

#[test]
fn no_displays_means_nothing_to_aim_at() {
    assert!(aim(rect(0.0, 0.0, 72.0, 32.0), &[]).is_err());
}
