use std::process::Command;
use std::time::{Duration, Instant};

use agent_desktop_core::{Deadline, ErrorCode, WindowFilter};

use super::interaction_lease::acquire;
use super::launch::launch_app;
use super::{SharedBus, list_windows};
use agent_desktop_core::launch_options::LaunchOptions;

fn deadline() -> Deadline {
    Deadline::after(10_000).expect("a deadline builds")
}

fn scoped(app: &str) -> WindowFilter {
    WindowFilter {
        focused_only: false,
        app: Some(app.to_owned()),
    }
}

fn signal(name: &str, pid: u32) {
    let _ = Command::new("kill")
        .args([name, &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status();
}

struct Running(u32);

impl Drop for Running {
    fn drop(&mut self) {
        signal("-CONT", self.0);
        signal("-KILL", self.0);
    }
}

fn check(alias: &str, executable: &str) {
    assert!(
        std::env::var_os("AGENT_DESKTOP_PRIVATE_SESSION").is_some(),
        "live launches must run inside scripts/private-session.sh"
    );
    let bus = SharedBus::default();
    let lease = acquire(deadline()).expect("a lease");
    let options = LaunchOptions {
        timeout_ms: 15_000,
        ..LaunchOptions::default()
    };
    let launched = launch_app(&bus, alias, &options, &lease).expect("launch by entry name");
    let pid = launched.pid.get();
    let running = Running(pid);
    assert_eq!(
        launched.app, executable,
        "the listed name never follows the alias"
    );
    signal("-STOP", pid);
    let stopped_at = Instant::now();
    let windows = list_windows(&bus, &scoped(alias), deadline()).expect_err("scoped listing");
    assert!(
        stopped_at.elapsed() < Duration::from_millis(250),
        "a stopped app is told from /proc, not after waiting out its probes: {:?}",
        stopped_at.elapsed()
    );
    assert_eq!(windows.code, ErrorCode::AppUnresponsive);
    drop(running);
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
fn alias_live_calculator_resolves_by_entry_name_and_stopped_is_unresponsive() {
    check("Calculator", "gnome-calculator");
}
