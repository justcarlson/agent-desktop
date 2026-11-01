use agent_desktop_core::{DeliverySemantics, ErrorCode};

use super::super::Request;
use super::super::fake::{Fake, window};
use super::super::graceful::close;
use super::Standing;
use crate::system::process::liveness::Liveness;
use crate::system::x11::control::ControlRead;
use crate::system::x11::window::toplevel::Toplevel;

fn request() -> Request<'static> {
    Request {
        name: "dialog_probe",
        pid: 42,
        instance: Some("t"),
        force: false,
    }
}

fn titled(id: u32, title: &str, transient: bool) -> Toplevel {
    Toplevel {
        title: title.to_owned(),
        transient,
        ..window(id)
    }
}

fn host_and_unparented_dialog() -> Fake {
    let mut world = Fake::new(Liveness::Alive);
    world.windows = ControlRead::Ready(vec![
        titled(1, "AD Dialog Host", false),
        titled(2, "AD Probe Dialog", false),
    ]);
    world.framed = vec!["AD Probe Dialog".to_owned()];
    world.exits_on_close = true;
    world
}

#[test]
fn a_dialog_frame_the_compositor_lists_without_a_parent_holds_every_close_request() {
    let world = host_and_unparented_dialog();
    let error = close(&world, &request()).expect_err("a dialog is open");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    let details = error.details.expect("details");
    assert_eq!(details["cause"], "dialog_pending");
    assert_eq!(details["dialogs"], 1);
    assert_eq!(details["windows_remaining"], 1);
    assert_eq!(details["dialog"], "AD Probe Dialog");
    assert_eq!(world.count("close"), 0);
}

#[test]
fn a_dialog_frame_with_a_parent_the_compositor_reports_counts_once() {
    let mut world = host_and_unparented_dialog();
    world.windows = ControlRead::Ready(vec![
        titled(1, "AD Dialog Host", false),
        titled(2, "AD Probe Dialog", true),
    ]);
    let error = close(&world, &request()).expect_err("a dialog is open");
    let details = error.details.expect("details");
    assert_eq!(details["dialogs"], 1);
    assert_eq!(details["windows_remaining"], 1);
    assert_eq!(world.count("close"), 0);
}

#[test]
fn a_dialog_frame_no_toplevel_matches_still_holds_every_close_request() {
    let mut world = host_and_unparented_dialog();
    world.windows = ControlRead::Ready(vec![titled(1, "AD Dialog Host", false)]);
    let error = close(&world, &request()).expect_err("a dialog is open");
    let details = error.details.expect("details");
    assert_eq!(details["cause"], "dialog_pending");
    assert_eq!(details["dialogs"], 1);
    assert_eq!(details["windows_remaining"], 1);
    assert_eq!(world.count("close"), 0);
}

#[test]
fn a_dialog_frame_sharing_its_title_with_a_parented_dialog_marks_that_dialog() {
    let mut world = host_and_unparented_dialog();
    world.windows = ControlRead::Ready(vec![
        titled(1, "Untitled", false),
        titled(2, "Untitled", true),
    ]);
    world.framed = vec!["Untitled".to_owned()];
    let standing = Standing::read(&world).expect("read");
    let dialogs: Vec<u32> = standing
        .windows
        .iter()
        .filter(|window| super::super::refusal::is_dialog(window))
        .map(|window| window.id)
        .collect();
    assert_eq!(dialogs, vec![2]);
    assert!(standing.nested.is_empty());
}

#[test]
fn without_a_dialog_frame_every_window_gets_a_close_request() {
    let mut world = host_and_unparented_dialog();
    world.framed.clear();
    close(&world, &request()).expect("closes");
    assert_eq!(world.count("close"), 2);
}
