use super::fakes::*;
use super::*;
use crate::system::x11::stacking_read::StackingRead;

fn covering() -> StackingRead {
    snapshot(
        vec![
            stacked(CLIENT, extents(101, 138, 1000, 700)),
            stacked(11, extents(500, 400, 100, 100)),
        ],
        Vec::new(),
    )
}

fn reason_of(error: &AdapterError) -> Option<&str> {
    error
        .details
        .as_ref()
        .and_then(|details| details["reason"].as_str())
}

#[test]
fn a_cover_without_a_redirect_route_refuses_and_issues_no_plain_grab() {
    let rig = Rig::standard();
    rig.stacking.reads.set(vec![covering()]);
    let error = capture(&rig.env(), &target(), false, deadline()).expect_err("covered");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(reason_of(&error), Some("covered"));
    assert_eq!(rig.grabber.grab_count(), 0);
    assert_eq!(rig.grabber.redirect_count(), 1);
}

#[test]
fn a_cover_is_read_through_the_redirect_and_never_by_a_plain_grab() {
    let rig = Rig::standard();
    rig.grabber.redirect_works();
    rig.stacking.reads.set(vec![covering()]);
    let image = capture(&rig.env(), &target(), false, deadline()).expect("captured");
    assert_eq!(rig.grabber.grab_count(), 0);
    assert_eq!(rig.grabber.redirect_count(), 1);
    assert_eq!(image.width, 1002);
}

#[test]
fn a_clear_window_is_a_plain_grab_and_never_a_redirect() {
    let rig = Rig::standard();
    rig.grabber.redirect_works();
    capture(&rig.env(), &target(), false, deadline()).expect("captured");
    assert_eq!(rig.grabber.grab_count(), 1);
    assert_eq!(rig.grabber.redirect_count(), 0);
}

#[test]
fn a_redirect_that_times_out_is_a_covered_window_not_a_deadline() {
    let rig = Rig::standard();
    rig.stacking.reads.set(vec![covering()]);
    *rig.grabber.redirect_with.lock().expect("redirect lock") =
        Box::new(|_, _| Err(crate::system::x11::grab_error::GrabError::Timeout));
    let error = capture(&rig.env(), &target(), false, deadline()).expect_err("covered");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(reason_of(&error), Some("covered"));
    assert!(
        error
            .suggestion
            .as_deref()
            .is_some_and(|text| text.contains("focus-window")),
        "{:?}",
        error.suggestion
    );
}

#[test]
fn the_same_cover_on_a_compositing_session_still_grabs() {
    let rig = Rig::standard();
    rig.compose_on();
    rig.stacking.reads.set(vec![covering()]);
    capture(&rig.env(), &target(), false, deadline()).expect("captured");
    assert_eq!(rig.grabber.grab_count(), 1);
    assert_eq!(rig.grabber.redirect_count(), 0);
}

#[test]
fn a_cover_that_appears_after_the_grab_discards_the_bytes() {
    let rig = Rig::standard();
    rig.stacking.reads.set(vec![clear_read(), covering()]);
    let error = capture(&rig.env(), &target(), false, deadline()).expect_err("covered");
    assert_eq!(reason_of(&error), Some("covered"));
    assert_eq!(rig.grabber.grab_count(), 1);
}

#[test]
fn unknown_stacking_takes_the_redirect_route_and_refuses_without_one() {
    let rig = Rig::standard();
    rig.stacking.reads.set(vec![StackingRead::Unavailable]);
    let error = capture(&rig.env(), &target(), false, deadline()).expect_err("unknown");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(rig.grabber.grab_count(), 0);
    let working = Rig::standard();
    working.grabber.redirect_works();
    working.stacking.reads.set(vec![StackingRead::Unavailable]);
    capture(&working.env(), &target(), false, deadline()).expect("captured");
    assert_eq!(working.grabber.redirect_count(), 1);
}

#[test]
fn stacking_is_not_read_on_a_compositing_session() {
    let rig = Rig::standard();
    rig.compose_on();
    capture(&rig.env(), &target(), false, deadline()).expect("captured");
    assert_eq!(
        rig.stacking
            .read_count
            .load(std::sync::atomic::Ordering::SeqCst),
        0
    );
}

fn unproven_bounds() -> Option<Rect> {
    Some(Rect {
        x: 100.0,
        y: 100.0,
        width: 1000.0,
        height: 900.0,
    })
}

#[test]
fn bounds_no_drawable_matches_still_return_the_frame_from_screenshot() {
    let rig = Rig::standard();
    rig.lookup
        .resolved
        .set(vec![Some(window_info(unproven_bounds()))]);
    capture(&rig.env(), &target(), false, deadline()).expect("captured");
    assert_eq!(rig.grabber.last_grab().0, FRAME);
}

#[test]
fn bounds_no_drawable_matches_refuse_the_debug_frame_without_a_grab() {
    let rig = Rig::standard();
    rig.lookup
        .resolved
        .set(vec![Some(window_info(unproven_bounds()))]);
    let error = capture(&rig.env(), &target(), true, deadline()).expect_err("unproven");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(rig.grabber.grab_count(), 0);
}

#[test]
fn missing_bounds_refuse_the_debug_frame_but_not_screenshot() {
    let rig = Rig::standard();
    rig.lookup.resolved.set(vec![Some(window_info(None))]);
    capture(&rig.env(), &target(), false, deadline()).expect("captured");
    let strict = Rig::standard();
    strict.lookup.resolved.set(vec![Some(window_info(None))]);
    assert!(capture(&strict.env(), &target(), true, deadline()).is_err());
}

#[test]
fn bounds_that_fit_the_frame_return_the_frame_to_the_debug_capture() {
    let rig = Rig::standard();
    let image = capture(&rig.env(), &target(), true, deadline()).expect("captured");
    assert_eq!(rig.grabber.last_grab().0, FRAME);
    assert_eq!(image.width, 1002);
}

#[test]
fn bounds_that_fit_only_the_client_pick_the_client() {
    let rig = Rig::standard();
    let fitting_client = Some(Rect {
        x: 100.0,
        y: 138.0,
        width: 1000.0,
        height: 700.0,
    });
    rig.lookup
        .resolved
        .set(vec![Some(window_info(fitting_client))]);
    capture(&rig.env(), &target(), true, deadline()).expect("captured");
    assert_eq!(rig.grabber.last_grab().0, CLIENT);
}

#[test]
fn an_image_over_the_pixel_cap_is_refused_before_the_grab() {
    let rig = Rig::standard();
    rig.grabber.describe_as(
        FRAME,
        vec![Drawn {
            rect: extents(0, 0, 9_000, 9_000),
            parent: None,
            state: 0,
        }],
    );
    let huge = Some(Rect {
        x: 0.0,
        y: 0.0,
        width: 9_000.0,
        height: 9_000.0,
    });
    rig.lookup.resolved.set(vec![Some(window_info(huge))]);
    let error = capture(&rig.env(), &target(), false, deadline()).expect_err("too large");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(rig.grabber.grab_count(), 0);
}

#[test]
fn an_unconvertible_visual_is_an_action_failure_with_no_image() {
    let rig = Rig::standard();
    let mut env = rig.env();
    env.decode = |_| {
        Err(AdapterError::new(
            ErrorCode::ActionFailed,
            "unsupported visual",
        ))
    };
    let error = capture(&env, &target(), false, deadline()).expect_err("unconvertible");
    assert_eq!(error.code, ErrorCode::ActionFailed);
}

fn two_displays() -> Vec<agent_desktop_core::DisplayInfo> {
    vec![
        display_at("left", 0.0, 1920.0, 1.0, true),
        display_at("right", 1920.0, 1920.0, 2.0, false),
    ]
}

#[test]
fn the_scale_comes_from_the_display_with_the_largest_overlap() {
    let rig = Rig::standard();
    rig.lookup.displays.set(vec![Some(two_displays())]);
    let straddling = Some(Rect {
        x: 1800.0,
        y: 100.0,
        width: 1000.0,
        height: 737.0,
    });
    rig.lookup.resolved.set(vec![Some(window_info(straddling))]);
    let image = capture(&rig.env(), &target(), false, deadline()).expect("captured");
    assert!((image.scale_factor - 2.0).abs() < f64::EPSILON);
}

#[test]
fn the_scale_is_one_when_displays_are_unavailable() {
    let rig = Rig::standard();
    rig.lookup.displays.set(vec![None]);
    let image = capture(&rig.env(), &target(), false, deadline()).expect("captured");
    assert!((image.scale_factor - 1.0).abs() < f64::EPSILON);
}
