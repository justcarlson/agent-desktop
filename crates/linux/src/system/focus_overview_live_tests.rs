use std::process::Command;
use std::time::{Duration, Instant};

use agent_desktop_core::{Deadline, ErrorCode, WindowFilter, WindowInfo};

use super::focus_window::focus_window;
use super::interaction_lease::acquire;
use super::overview::overview_open;
use super::{SharedBus, list_windows};
use crate::tree::live_launch::Launched;

const SETTLE: Duration = Duration::from_secs(40);

const NEVER_FOCUSED: &str = r#"
import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk
w = Gtk.Window(title="adov")
w.set_default_size(320, 200)
w.set_focus_on_map(False)
w.add(Gtk.Button(label="b"))
w.connect("destroy", Gtk.main_quit)
w.show_all()
Gtk.main()
"#;

fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).expect("a deadline builds")
}

fn key(name: &str) {
    let home = std::env::var("HOME").expect("HOME");
    let status = Command::new(format!("{home}/.local/ad-tools/root/usr/bin/xdotool"))
        .env(
            "LD_LIBRARY_PATH",
            format!("{home}/.local/ad-tools/root/usr/lib/x86_64-linux-gnu"),
        )
        .args(["key", name])
        .status()
        .expect("xdotool runs");
    assert!(status.success());
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

fn window(bus: &SharedBus, pid: u32) -> WindowInfo {
    let everything = WindowFilter {
        focused_only: false,
        app: None,
    };
    let started = Instant::now();
    while started.elapsed() < SETTLE {
        let found = list_windows(bus, &everything, deadline(10_000))
            .unwrap_or_default()
            .into_iter()
            .find(|window| window.pid == pid);
        if let Some(window) = found {
            return window;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("the window was never listed");
}

fn cause_of(error: &agent_desktop_core::AdapterError) -> String {
    error
        .details
        .as_ref()
        .and_then(|details| details["cause"].as_str())
        .unwrap_or("none")
        .to_owned()
}

fn focus(window: &WindowInfo) -> Result<(), agent_desktop_core::AdapterError> {
    let lease = acquire(deadline(8_000)).expect("a lease");
    focus_window(&SharedBus::default(), window, &lease)
}

#[test]
#[ignore = "drives a real desktop session; run through scripts/private-session.sh, or a private gnome-shell with AGENT_DESKTOP_LIVE_WM=mutter"]
fn focus_window_names_an_open_overview_and_succeeds_once_it_is_closed() {
    let mutter = std::env::var("AGENT_DESKTOP_LIVE_WM").as_deref() == Ok("mutter");
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let dir = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("a private session");
    let path = format!("{dir}/adov.py");
    std::fs::write(&path, NEVER_FOCUSED).expect("a script writes");
    let pid = launched.spawn("python3", &[&path]);
    let target = window(&bus, pid);
    if !mutter {
        assert_eq!(overview_open(), None, "no GNOME Shell, no overview");
        focus(&target).expect("a window manager without an overview grants focus");
        return;
    }
    if overview_open() != Some(true) {
        key("super");
        wait_for_overview(true);
    }
    let refused = focus(&target).expect_err("no activation while the overview is open");
    assert_eq!(refused.code, ErrorCode::ActionFailed);
    assert_eq!(cause_of(&refused), "overview_open");
    key("Escape");
    wait_for_overview(false);
    focus(&target).expect("a window that never held focus is granted once the overview is closed");
    key("super");
    wait_for_overview(true);
    let held = focus(&target).expect_err("no activation while the overview is open");
    assert_eq!(
        cause_of(&held),
        "overview_open",
        "the window was already active: {held:?}"
    );
    key("Escape");
    wait_for_overview(false);
}
