use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode, Point};

use super::*;

fn at(x: f64, y: f64) -> Point {
    Point { x, y }
}

fn refusal_cause(error: &AdapterError) -> Option<&str> {
    error.details.as_ref()?.get("cause")?.as_str()
}

#[test]
fn a_first_motion_passes_one_pixel_beside_the_aim_and_ends_on_it() {
    assert_eq!(
        relative_moves(&at(100.0, 50.0), (953, 81), true, -1),
        vec![(852.0, 31.0), (1.0, 0.0)],
        "every motion must be relative: XWayland sends an absolute-only motion through another device than the wheel, and GDK3 then drops the next notch"
    );
}

#[test]
fn a_first_motion_onto_the_resting_cursor_still_moves_it() {
    assert_eq!(
        relative_moves(&at(953.0, 81.0), (953, 81), true, -1),
        vec![(-1.0, 0.0), (1.0, 0.0)]
    );
}

#[test]
fn a_cursor_already_beside_the_aim_skips_the_empty_motion() {
    assert_eq!(
        relative_moves(&at(952.0, 81.0), (953, 81), true, -1),
        vec![(1.0, 0.0)]
    );
}

#[test]
fn a_later_motion_goes_straight_to_its_aim() {
    assert_eq!(
        relative_moves(&at(953.0, 81.0), (960, 90), false, -1),
        vec![(7.0, 9.0)]
    );
    assert!(relative_moves(&at(960.0, 90.0), (960, 90), false, -1).is_empty());
}

#[test]
fn a_fractional_cursor_moves_by_its_fraction() {
    assert_eq!(
        relative_moves(&at(10.5, 20.25), (12, 20), false, -1),
        vec![(1.5, -0.25)]
    );
}

#[test]
fn a_cursor_on_the_aim_lands() {
    assert!(confirm_landed(&at(953.0, 81.0), (953, 81)).is_ok());
    assert!(confirm_landed(&at(953.4, 80.6), (953, 81)).is_ok());
}

#[test]
fn a_cursor_off_the_aim_refuses_as_not_delivered() {
    for landed in [
        at(954.0, 81.0),
        at(953.0, 79.0),
        at(0.0, 0.0),
        at(953.5, 81.0),
    ] {
        let error = confirm_landed(&landed, (953, 81)).expect_err(
            "an accelerated, clamped or rounded motion that missed the aim must refuse",
        );
        assert_eq!(error.code, ErrorCode::ActionFailed);
        assert_eq!(refusal_cause(&error), Some("cursor_missed_aim"));
        assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    }
}

#[test]
fn an_unread_cursor_refuses_as_not_delivered() {
    let error = no_cursor_evidence(AdapterError::internal("no reply"));
    assert_eq!(refusal_cause(&error), Some("cursor_unread"));
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
}
