use std::time::{Duration, Instant};

use agent_desktop_core::ErrorCode;

use super::SharedBus;
use super::focus_window::lift_live_tests::{focus, spawn_with, tool};
use super::overview::overview_open;
use crate::tree::live_launch::Launched;

fn key(name: &str) {
    assert!(
        tool("xdotool")
            .args(["key", name])
            .status()
            .expect("xdotool runs")
            .success()
    );
}

fn wait_for_overview(open: bool) {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(8) {
        if overview_open() == Some(open) {
            return;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    panic!("the overview never read {open}");
}

fn cause_of(error: &agent_desktop_core::AdapterError) -> String {
    error
        .details
        .as_ref()
        .and_then(|details| details["cause"].as_str())
        .unwrap_or("none")
        .to_owned()
}

#[test]
#[ignore = "drives a real desktop session; run through scripts/private-session.sh, or a private gnome-shell with AGENT_DESKTOP_LIVE_WM=mutter"]
fn focus_window_names_an_open_overview_and_succeeds_once_it_is_closed() {
    let mutter = std::env::var("AGENT_DESKTOP_LIVE_WM").as_deref() == Ok("mutter");
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let target = spawn_with(&mut launched, &bus, "overview", true);
    if !mutter {
        assert_eq!(overview_open(), None, "no GNOME Shell, no overview");
        focus(&bus, &target).expect("a window manager without an overview grants focus");
        return;
    }
    if overview_open() != Some(true) {
        key("super");
        wait_for_overview(true);
    }
    let started = Instant::now();
    let refused = focus(&bus, &target).expect_err("no activation while the overview is open");
    let refusal_ms = started.elapsed().as_millis();
    assert_eq!(refused.code, ErrorCode::ActionFailed);
    assert_eq!(cause_of(&refused), "overview_open");
    assert!(
        refusal_ms < 2_000,
        "an open Overview should be identified promptly, took {refusal_ms}ms"
    );
    assert!(refused.suggestion.as_deref().is_some_and(|text| {
        text.contains("Escape") && text.contains("Close the Activities Overview")
    }));
    key("Escape");
    wait_for_overview(false);
    focus(&bus, &target)
        .expect("a window that never held focus is granted once the overview is closed");
    key("super");
    wait_for_overview(true);
    let held = focus(&bus, &target).expect_err("no activation while the overview is open");
    assert_eq!(
        cause_of(&held),
        "overview_open",
        "the window was already active: {held:?}"
    );
    key("Escape");
    wait_for_overview(false);
}
