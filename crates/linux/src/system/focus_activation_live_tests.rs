use std::process::Command;
use std::time::{Duration, Instant};

use agent_desktop_core::{Deadline, WindowFilter, WindowInfo};

use super::focus_window::focus_window;
use super::interaction_lease::acquire;
use super::{SharedBus, list_windows};
use crate::tree::live_launch::Launched;

const SETTLE: Duration = Duration::from_secs(40);

fn script(title: &str) -> String {
    format!(
        r#"
import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk
w = Gtk.Window(title="{title}")
w.set_default_size(320, 200)
w.add(Gtk.Button(label="b"))
w.connect("destroy", Gtk.main_quit)
w.show_all()
Gtk.main()
"#
    )
}

fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).expect("a deadline builds")
}

fn spawn(launched: &mut Launched, title: &str) -> u32 {
    let dir = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("a private session");
    let path = format!("{dir}/adact-{title}.py");
    std::fs::write(&path, script(title)).expect("a script writes");
    launched.spawn("python3", &[&path])
}

fn window_titled(bus: &SharedBus, pid: u32, title: &str) -> WindowInfo {
    let everything = WindowFilter {
        focused_only: false,
        app: None,
    };
    let started = Instant::now();
    while started.elapsed() < SETTLE {
        let found = list_windows(bus, &everything, deadline(10_000))
            .unwrap_or_default()
            .into_iter()
            .find(|window| window.pid == pid && window.title == title);
        if let Some(window) = found {
            return window;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("{title} never listed");
}

fn is_focused(bus: &SharedBus, id: &str) -> bool {
    let everything = WindowFilter {
        focused_only: false,
        app: None,
    };
    list_windows(bus, &everything, deadline(10_000))
        .unwrap_or_default()
        .iter()
        .any(|window| window.id == id && window.state.is_focused)
}

fn x_id_of(title: &str) -> String {
    let tools = format!(
        "{}/.local/ad-tools/root/usr/bin/wmctrl",
        std::env::var("HOME").expect("HOME")
    );
    let listing = Command::new(&tools)
        .arg("-l")
        .output()
        .expect("wmctrl runs");
    String::from_utf8_lossy(&listing.stdout)
        .lines()
        .find(|line| line.ends_with(title))
        .and_then(|line| line.split_whitespace().next())
        .map(str::to_owned)
        .unwrap_or_else(|| panic!("no X window titled {title}"))
}

fn activate_externally(x_id: &str) {
    let tools = format!(
        "{}/.local/ad-tools/root/usr/bin/wmctrl",
        std::env::var("HOME").expect("HOME")
    );
    let status = Command::new(tools)
        .args(["-i", "-a", x_id])
        .status()
        .expect("wmctrl runs");
    assert!(status.success());
    std::thread::sleep(Duration::from_millis(500));
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
fn focus_window_takes_focus_back_from_a_window_another_tool_activated() {
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let first_pid = spawn(&mut launched, "adactA");
    let first = window_titled(&bus, first_pid, "adactA");
    let second_pid = spawn(&mut launched, "adactB");
    let _second = window_titled(&bus, second_pid, "adactB");
    let other = x_id_of("adactB");
    for round in 0..10 {
        activate_externally(&other);
        assert!(
            !is_focused(&bus, &first.id),
            "round {round}: A already focused"
        );
        let lease = acquire(deadline(8_000)).expect("a lease");
        let outcome = focus_window(&SharedBus::default(), &first, &lease);
        assert!(outcome.is_ok(), "round {round}: {outcome:?}");
        assert!(is_focused(&bus, &first.id), "round {round}: A not focused");
    }
}
