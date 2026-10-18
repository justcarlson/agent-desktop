use agent_desktop_core::ErrorCode;

use super::decide;
use crate::system::display_mutter::LogicalMonitor;
use crate::system::x11::{DisplayServer, X11Monitor};
use crate::tree::Extents;

fn monitor(name: &str, connectors: &[&str], x: i32, w: i32, primary: bool) -> X11Monitor {
    X11Monitor {
        name: name.to_owned(),
        connectors: connectors.iter().map(|c| c.to_string()).collect(),
        bounds: Extents {
            x,
            y: 0,
            width: w,
            height: 800,
        },
        primary,
    }
}

fn logical(connectors: &[&str], scale: f64, primary: bool) -> LogicalMonitor {
    LogicalMonitor {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
        scale,
        primary,
        connectors: connectors.iter().map(|c| c.to_string()).collect(),
    }
}

fn x11() -> DisplayServer {
    DisplayServer::X11(":0".to_owned())
}

#[test]
fn the_measured_virtual_monitor_reports_name_bounds_primary_and_one_scale() {
    let displays = decide(
        x11(),
        Some(vec![monitor("Virtual1", &["Virtual1"], 0, 1280, true)]),
        None,
    )
    .expect("monitors read");
    let [display] = displays.as_slice() else {
        panic!("one display")
    };
    assert_eq!(display.id, "Virtual1");
    assert_eq!(
        (
            display.bounds.x,
            display.bounds.width,
            display.bounds.height
        ),
        (0.0, 1280.0, 800.0)
    );
    assert!(display.is_primary);
    assert_eq!(display.scale, 1.0);
}

#[test]
fn mutter_scale_matches_on_a_shared_connector_and_missing_monitors_keep_one() {
    let monitors = vec![
        monitor("DP-1", &["DP-1"], 0, 1920, true),
        monitor("HDMI-1", &["HDMI-1"], 1920, 1920, false),
    ];
    let mutter = vec![logical(&["DP-1"], 2.0, true)];
    let displays = decide(x11(), Some(monitors), Some(mutter)).expect("read");
    assert_eq!(displays[0].scale, 2.0);
    assert_eq!(displays[1].scale, 1.0);
}

#[test]
fn mutter_scale_also_matches_a_randr_monitor_named_for_its_output() {
    let monitors = vec![monitor("auto", &["eDP-1"], 0, 1920, true)];
    let mutter = vec![logical(&["eDP-1"], 1.5, true)];
    let displays = decide(x11(), Some(monitors), Some(mutter)).expect("read");
    assert_eq!(displays[0].scale, 1.5);
}

#[test]
fn the_primary_monitor_sorts_first_even_when_listed_last() {
    let monitors = vec![
        monitor("HDMI-1", &["HDMI-1"], 1920, 1920, false),
        monitor("DP-1", &["DP-1"], 0, 1920, true),
    ];
    let displays = decide(x11(), Some(monitors), None).expect("read");
    assert!(displays[0].is_primary);
    assert_eq!(displays[0].id, "DP-1");
}

#[test]
fn an_x11_read_failure_is_an_error_not_an_empty_list() {
    let error = decide(x11(), None, None).expect_err("unreadable");
    assert_eq!(error.code, ErrorCode::Internal);
}

#[test]
fn a_wayland_session_without_mutter_is_platform_not_supported() {
    let error = decide(DisplayServer::Wayland, Some(vec![]), None).expect_err("no mutter");
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
}

#[test]
fn a_wayland_session_uses_the_mutter_list_alone() {
    let mutter = vec![logical(&["XWAYLAND0"], 1.0, true)];
    let displays = decide(
        DisplayServer::Wayland,
        Some(vec![monitor("ignored", &[], 0, 1, true)]),
        Some(mutter),
    )
    .expect("mutter answers");
    let [display] = displays.as_slice() else {
        panic!("one display")
    };
    assert_eq!(display.id, "XWAYLAND0");
    assert_eq!(display.bounds.width, 1920.0);
}

#[test]
fn a_session_without_a_display_server_is_platform_not_supported() {
    let error = decide(DisplayServer::None, None, None).expect_err("headless");
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
}
