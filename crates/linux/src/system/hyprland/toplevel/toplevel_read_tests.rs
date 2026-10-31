use agent_desktop_core::{Deadline, ErrorCode};

use super::*;
use crate::system::hyprland::toplevel::fake_binding::binding_for;
use crate::system::wayland::fake_compositor::{FakeCompositor, Script};
use crate::system::wayland::fake_toplevels::FakeToplevel;

const BOTH: [(&str, u32); 2] = [(LIST_INTERFACE, 1), (MAPPING_INTERFACE, 1)];

fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).expect("deadline")
}

fn toplevel(identifier: &'static str, address: Option<u64>) -> FakeToplevel {
    FakeToplevel {
        identifier,
        address,
        ..FakeToplevel::default()
    }
}

#[test]
fn every_mapped_toplevel_is_read_with_its_full_hyprland_address() {
    let compositor = FakeCompositor::start(Script::Toplevels {
        globals: BOTH.to_vec(),
        toplevels: vec![
            toplevel("a1", Some(0x5555_0000_0010)),
            toplevel("b2", Some(0x20)),
        ],
    });
    let views = read_toplevels(&binding_for(&compositor), deadline(2_000)).expect("reads");
    assert_eq!(
        views,
        vec![
            ToplevelView {
                identifier: "a1".into(),
                address: 0x5555_0000_0010,
            },
            ToplevelView {
                identifier: "b2".into(),
                address: 0x20,
            },
        ]
    );
}

#[test]
fn a_toplevel_whose_mapping_failed_is_left_out() {
    let compositor = FakeCompositor::start(Script::Toplevels {
        globals: BOTH.to_vec(),
        toplevels: vec![toplevel("a1", None), toplevel("b2", Some(0x20))],
    });
    let views = read_toplevels(&binding_for(&compositor), deadline(2_000)).expect("reads");
    let identifiers: Vec<&str> = views.iter().map(|view| view.identifier.as_str()).collect();
    assert_eq!(identifiers, vec!["b2"]);
}

#[test]
fn two_toplevels_mapped_to_one_address_refuse_the_read() {
    let compositor = FakeCompositor::start(Script::Toplevels {
        globals: BOTH.to_vec(),
        toplevels: vec![toplevel("a1", Some(0x20)), toplevel("b2", Some(0x20))],
    });
    let error = read_toplevels(&binding_for(&compositor), deadline(2_000)).expect_err("refuses");
    assert_eq!(error.code, ErrorCode::Internal);
}

#[test]
fn a_compositor_without_either_toplevel_global_is_not_supported() {
    for (missing, present) in [(LIST_INTERFACE, BOTH[1]), (MAPPING_INTERFACE, BOTH[0])] {
        let compositor = FakeCompositor::start(Script::Toplevels {
            globals: vec![present],
            toplevels: vec![toplevel("a1", Some(0x10))],
        });
        let error =
            read_toplevels(&binding_for(&compositor), deadline(2_000)).expect_err("refuses");
        assert_eq!(error.code, ErrorCode::PlatformNotSupported);
        assert_eq!(
            error.platform_detail.as_deref(),
            Some(format!("missing Wayland global {missing}").as_str())
        );
    }
}

#[test]
fn a_socket_served_by_a_restarted_compositor_is_never_read() {
    let compositor = FakeCompositor::start(Script::Toplevels {
        globals: BOTH.to_vec(),
        toplevels: vec![toplevel("a1", Some(0x10))],
    });
    let mut binding = binding_for(&compositor);
    binding.start_time += 1;
    let error = read_toplevels(&binding, deadline(2_000)).expect_err("refuses");
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert_eq!(
        error
            .details
            .as_ref()
            .and_then(|details| details.get("display_unavailable"))
            .and_then(|cause| cause.as_str()),
        Some("hyprland_mismatch")
    );
}

#[test]
fn a_silent_compositor_times_out_by_the_deadline() {
    let compositor = FakeCompositor::start(Script::Silent);
    let error = read_toplevels(&binding_for(&compositor), deadline(200)).expect_err("silent");
    assert_eq!(error.code, ErrorCode::Timeout);
}
