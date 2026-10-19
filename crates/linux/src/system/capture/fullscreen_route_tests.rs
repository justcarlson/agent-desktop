use agent_desktop_core::{ErrorCode, ScreenshotTarget};

use super::*;
use crate::system::capture::display;
use crate::system::capture::window::fakes::*;

#[test]
fn a_non_compositing_session_takes_the_root_and_a_compositing_one_is_refused() {
    assert_eq!(choose(false), FullRoute::Root);
    assert_eq!(choose(true), FullRoute::Refused);
}

#[test]
fn a_non_compositing_screen_is_one_root_grab() {
    let rig = Rig::standard();
    display::capture(&rig.env(), &ScreenshotTarget::FullScreen, deadline()).expect("captured");
    assert_eq!(rig.grabber.grab_count(), 1);
    assert_eq!(rig.grabber.last_grab().0, ROOT_DRAWABLE);
}

#[test]
fn a_compositing_screen_is_refused_naming_the_cause_and_issues_no_grab() {
    let rig = Rig::standard();
    rig.compose_on();
    let error = display::capture(&rig.env(), &ScreenshotTarget::FullScreen, deadline())
        .expect_err("refused");
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert_eq!(
        error.details.as_ref().and_then(|d| d["reason"].as_str()),
        Some("compositing")
    );
    assert_eq!(rig.grabber.grab_count(), 0);
}

#[test]
fn every_display_target_is_refused_the_same_way_under_a_compositor() {
    for target in [ScreenshotTarget::Screen(0), ScreenshotTarget::FullScreen] {
        let rig = Rig::standard();
        rig.compose_on();
        let error = display::capture(&rig.env(), &target, deadline()).expect_err("refused");
        assert_eq!(error.code, ErrorCode::PlatformNotSupported);
        assert_eq!(rig.grabber.grab_count(), 0);
    }
}

#[test]
fn a_root_grab_failure_keeps_its_own_code() {
    let rig = Rig::standard();
    *rig.grabber.grab_with.lock().expect("grab lock") =
        Box::new(|_, _| Err(crate::system::x11::grab_error::GrabError::Timeout));
    let error = display::capture(&rig.env(), &ScreenshotTarget::FullScreen, deadline())
        .expect_err("timeout");
    assert_eq!(error.code, ErrorCode::Timeout);
}
