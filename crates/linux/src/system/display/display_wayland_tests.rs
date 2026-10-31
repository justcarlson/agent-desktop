use std::cell::Cell;

use agent_desktop_core::{Deadline, ErrorCode};

use super::*;
use crate::system::wayland::fake_compositor::{FakeCompositor, FakeOutput, Script};

fn output(name: &str, x: i32, y: i32) -> WaylandOutput {
    WaylandOutput {
        name: Some(name.to_owned()),
        position: Some((x, y)),
        logical: Some((1920, 1080)),
        mode: Some((1920, 1080)),
        turned: false,
        integer_scale: 1,
    }
}

fn ids(displays: &[DisplayInfo]) -> Vec<(&str, bool)> {
    displays
        .iter()
        .map(|display| (display.id.as_str(), display.is_primary))
        .collect()
}

fn deadline() -> Deadline {
    Deadline::after(2_000).expect("deadline")
}

fn fake(name: &'static str, position: (i32, i32)) -> FakeOutput {
    FakeOutput {
        name,
        position,
        logical: (1280, 800),
        mode: (2560, 1600),
        transform: 0,
        scale: 2,
    }
}

#[test]
fn the_output_at_the_layout_origin_is_primary_and_sorts_first() {
    let outputs = [
        output("HDMI-A-1", -1920, 0),
        output("eDP-1", 0, 0),
        output("DP-1", 1920, 0),
    ];
    let displays = from_outputs(&outputs).expect("complete");
    assert_eq!(
        ids(&displays),
        vec![("eDP-1", true), ("DP-1", false), ("HDMI-A-1", false)]
    );
}

#[test]
fn without_an_output_at_the_origin_the_top_left_one_is_primary() {
    let outputs = [
        output("DP-2", 1920, 100),
        output("DP-1", 0, 100),
        output("DP-3", 5000, 0),
    ];
    let displays = from_outputs(&outputs).expect("complete");
    assert_eq!(
        ids(&displays),
        vec![("DP-3", true), ("DP-1", false), ("DP-2", false)]
    );
}

#[test]
fn the_order_follows_the_names_not_the_advertisement_order() {
    let forward = [
        output("eDP-1", 0, 0),
        output("DP-1", 1920, 0),
        output("DP-2", 3840, 0),
    ];
    let backward = [
        output("DP-2", 3840, 0),
        output("DP-1", 1920, 0),
        output("eDP-1", 0, 0),
    ];
    assert_eq!(from_outputs(&forward), from_outputs(&backward));
}

#[test]
fn one_incomplete_output_makes_the_list_unusable() {
    let mut unnamed = output("DP-1", 1920, 0);
    unnamed.name = None;
    assert_eq!(from_outputs(&[output("eDP-1", 0, 0), unnamed]), None);
}

#[test]
fn mutter_answers_first_and_the_output_globals_are_never_read() {
    let asked = Cell::new(false);
    let mutter = vec![LogicalMonitor {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
        scale: 1.0,
        primary: true,
        connectors: vec!["Virtual-1".to_owned()],
    }];
    let displays = decide_wayland(Some(mutter), || {
        asked.set(true);
        Ok(Vec::new())
    })
    .expect("mutter answers");
    assert_eq!(ids(&displays), vec![("Virtual-1", true)]);
    assert!(!asked.get());
}

#[test]
fn without_mutter_the_output_globals_answer() {
    let displays = decide_wayland(None, || {
        Ok(from_outputs(&[output("DP-1", 0, 0)]).expect("complete"))
    })
    .expect("outputs answer");
    assert_eq!(ids(&displays), vec![("DP-1", true)]);
}

#[test]
fn a_compositor_with_the_output_globals_lists_its_displays() {
    let compositor = FakeCompositor::start(Script::Answer {
        globals: vec![
            ("wl_output", 4),
            ("wl_output", 4),
            ("zxdg_output_manager_v1", 3),
        ],
        outputs: vec![fake("DP-1", (1280, 0)), fake("eDP-1", (0, 0))],
    });
    let displays = list_outputs(&compositor.socket, deadline()).expect("listed");
    assert_eq!(ids(&displays), vec![("eDP-1", true), ("DP-1", false)]);
    assert_eq!(displays[1].bounds.x, 1280.0);
    assert_eq!(displays[1].bounds.width, 1280.0);
    assert_eq!(displays[1].scale, 2.0);
}

#[test]
fn a_compositor_without_xdg_output_keeps_the_platform_refusal_and_names_the_global() {
    let compositor = FakeCompositor::start(Script::Answer {
        globals: vec![("wl_output", 4)],
        outputs: vec![fake("DP-1", (0, 0))],
    });
    let error = list_outputs(&compositor.socket, deadline()).expect_err("refused");
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert_eq!(
        error.message,
        "Displays cannot be listed on this Wayland session"
    );
    assert!(
        error
            .suggestion
            .as_deref()
            .is_some_and(|text| text.contains("zxdg_output_manager_v1") && text.contains("GNOME"))
    );
    assert_eq!(
        error.platform_detail.as_deref(),
        Some("the compositor does not advertise zxdg_output_manager_v1")
    );
}

#[test]
fn a_compositor_that_names_no_output_is_refused_rather_than_listed_without_ids() {
    let compositor = FakeCompositor::start(Script::Answer {
        globals: vec![("zxdg_output_manager_v1", 1), ("wl_output", 3)],
        outputs: vec![fake("DP-1", (0, 0))],
    });
    let error = list_outputs(&compositor.socket, deadline()).expect_err("refused");
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert!(
        error
            .platform_detail
            .as_deref()
            .is_some_and(|detail| detail.contains("without a name"))
    );
}

#[test]
fn a_silent_compositor_is_a_retryable_timeout() {
    let compositor = FakeCompositor::start(Script::Silent);
    let error = list_outputs(&compositor.socket, Deadline::after(150).expect("deadline"))
        .expect_err("silent");
    assert_eq!(error.code, ErrorCode::Timeout);
    assert_eq!(error.details.expect("details")["retryable"], true);
}

#[test]
fn a_socket_nobody_serves_is_a_missing_display_server() {
    let compositor = FakeCompositor::start(Script::HangUp);
    let socket = compositor.socket.clone();
    drop(compositor);
    let error = list_outputs(&socket, deadline()).expect_err("gone");
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert_eq!(
        error.details.expect("details")["display_unavailable"],
        "missing"
    );
}
