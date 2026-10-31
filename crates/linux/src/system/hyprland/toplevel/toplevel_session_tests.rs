use agent_desktop_core::{Deadline, ErrorCode};

use super::*;
use crate::system::hyprland::toplevel::fake_binding::binding_for;
use crate::system::wayland::fake_compositor::{FakeCompositor, Script};
use crate::system::wayland::fake_toplevels::FakeToplevel;

const GLOBALS: [(&str, u32); 3] = [
    (SEAT_INTERFACE, 7),
    (WLR_MANAGER_INTERFACE, 3),
    (MAPPING_INTERFACE, 1),
];

fn toplevel(identifier: &'static str, address: Option<u64>) -> FakeToplevel {
    FakeToplevel {
        identifier,
        address,
        ..FakeToplevel::default()
    }
}

fn open(toplevels: Vec<FakeToplevel>) -> (FakeCompositor, ToplevelSession) {
    let compositor = FakeCompositor::start(Script::Toplevels {
        globals: GLOBALS.to_vec(),
        toplevels,
    });
    let deadline = Deadline::after(2_000).expect("deadline");
    let session = ToplevelSession::open(&binding_for(&compositor), deadline).expect("opens");
    (compositor, session)
}

fn sent(compositor: &FakeCompositor) -> Vec<String> {
    compositor.log.lock().expect("log").clone()
}

#[test]
fn every_mapped_live_toplevel_is_listed_with_its_address_state_and_parent() {
    let (_compositor, session) = open(vec![
        FakeToplevel {
            activated: true,
            ..toplevel("main", Some(0x5555_0000_0010))
        },
        toplevel("unmapped", None),
        FakeToplevel {
            parent: Some(0),
            ..toplevel("save", Some(0x20))
        },
    ]);
    let windows = session.windows();
    let seen: Vec<(u64, &str, bool, bool)> = windows
        .iter()
        .map(|w| (w.address, w.title.as_str(), w.activated, w.child))
        .collect();
    assert_eq!(
        seen,
        vec![
            (0x5555_0000_0010, "main", true, false),
            (0x20, "save", false, true),
        ]
    );
}

#[test]
fn an_activation_goes_to_the_handle_mapped_to_the_address_and_its_answer_is_read() {
    let (compositor, mut session) = open(vec![
        FakeToplevel {
            activated: true,
            ..toplevel("one", Some(0x10))
        },
        toplevel("two", Some(0x20)),
    ]);
    session.activate(0x20).expect("sends");
    session.settle().expect("settles");
    let activated: Vec<u64> = session
        .windows()
        .iter()
        .filter(|w| w.activated)
        .map(|w| w.address)
        .collect();
    assert_eq!(activated, vec![0x20]);
    assert_eq!(sent(&compositor), vec!["activate two".to_owned()]);
}

#[test]
fn an_address_with_no_live_handle_is_refused_before_anything_is_sent() {
    let (compositor, mut session) = open(vec![toplevel("one", Some(0x10))]);
    let error = session.activate(0x99).expect_err("refuses");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    session.settle().expect("settles");
    assert!(sent(&compositor).is_empty());
}

#[test]
fn a_close_reaches_its_handle_and_the_closed_toplevel_leaves_the_list() {
    let (compositor, mut session) = open(vec![
        toplevel("one", Some(0x10)),
        toplevel("two", Some(0x20)),
    ]);
    let id = session.windows()[0].id;
    assert!(session.close(id).expect("sends"));
    session.settle().expect("settles");
    let left: Vec<u64> = session.windows().iter().map(|w| w.address).collect();
    assert_eq!(left, vec![0x20]);
    assert!(!session.close(id).expect("answers"));
    assert_eq!(sent(&compositor), vec!["close one".to_owned()]);
}

#[test]
fn a_compositor_without_the_wlr_manager_is_not_supported() {
    let compositor = FakeCompositor::start(Script::Toplevels {
        globals: vec![GLOBALS[0], GLOBALS[2]],
        toplevels: vec![toplevel("one", Some(0x10))],
    });
    let deadline = Deadline::after(2_000).expect("deadline");
    let error = ToplevelSession::open(&binding_for(&compositor), deadline)
        .err()
        .expect("refuses");
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert_eq!(
        error.platform_detail.as_deref(),
        Some("missing Wayland global zwlr_foreign_toplevel_manager_v1")
    );
}
