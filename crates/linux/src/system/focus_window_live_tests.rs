use std::process::Command;
use std::time::{Duration, Instant};

use agent_desktop_core::{Deadline, WindowFilter, WindowInfo};

use super::focus_window::focus_window;
use super::interaction_lease::acquire;
use super::x11::stacking_read::StackingRead;
use super::x11::stacking_source::StackingSource;
use super::x11::{DisplayReader, X11State};
use super::{SharedBus, list_windows};
use crate::tree::live_launch::Launched;

const SETTLE: Duration = Duration::from_secs(40);

const GTK3: &str = r#"
import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk
w = Gtk.Window(title="adf")
w.set_default_size(320, 200)
w.add(Gtk.Button(label="b"))
w.connect("destroy", Gtk.main_quit)
w.show_all()
Gtk.main()
"#;

const GTK4: &str = r#"
import gi
gi.require_version("Gtk", "4.0")
from gi.repository import Gtk, GLib
Gtk.init()
loop = GLib.MainLoop()
w = Gtk.Window(title="adf")
w.set_default_size(320, 200)
w.set_child(Gtk.Button(label="b"))
w.connect("close-request", lambda *a: loop.quit())
w.present()
loop.run()
"#;

const QT: &str = r#"
import sys
from PyQt6.QtWidgets import QApplication, QMainWindow, QPushButton
app = QApplication(sys.argv)
w = QMainWindow()
w.setWindowTitle("adf")
w.setCentralWidget(QPushButton("b"))
w.resize(320, 200)
w.show()
sys.exit(app.exec())
"#;

const POPUP: &str = r#"
import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk
w = Gtk.Window(type=Gtk.WindowType.POPUP)
w.set_default_size(200, 100)
w.move(50, 50)
w.add(Gtk.Label(label="cover"))
w.connect("destroy", Gtk.main_quit)
w.show_all()
Gtk.main()
"#;

fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).expect("a deadline builds")
}

fn script_path(name: &str, source: &str) -> String {
    let dir = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("a private session");
    let path = format!("{dir}/{name}.py");
    std::fs::write(&path, source).expect("a script writes");
    path
}

fn spawn(launched: &mut Launched, kind: &str) -> u32 {
    let script = script_path(
        &format!("adf-{kind}"),
        match kind {
            "gtk3" => GTK3,
            "gtk4" => GTK4,
            _ => QT,
        },
    );
    if kind == "qt" {
        let home = std::env::var("HOME").expect("HOME");
        let python = format!("{home}/.local/ad-tools/qtvenv/bin/python");
        let libraries = format!("{home}/.local/ad-tools/root/usr/lib/x86_64-linux-gnu");
        return launched.spawn_with_env(
            &python,
            &[&script],
            &[
                ("QT_ACCESSIBILITY", "1".to_owned()),
                ("QT_LINUX_ACCESSIBILITY_ALWAYS_ON", "1".to_owned()),
                ("QT_QPA_PLATFORM", "xcb".to_owned()),
                ("LD_LIBRARY_PATH", libraries),
            ],
        );
    }
    launched.spawn("python3", &[&script])
}

fn windows(bus: &SharedBus) -> Vec<WindowInfo> {
    let everything = WindowFilter {
        focused_only: false,
        app: None,
    };
    list_windows(bus, &everything, deadline(10_000)).unwrap_or_default()
}

fn wait_for(bus: &SharedBus, pid: u32) -> WindowInfo {
    let started = Instant::now();
    while started.elapsed() < SETTLE {
        if let Some(window) = windows(bus).into_iter().find(|w| w.pid == pid) {
            return window;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("pid {pid} never listed a window");
}

fn focus(window: &WindowInfo) -> Result<(), agent_desktop_core::AdapterError> {
    let lease = acquire(deadline(8_000)).expect("a lease");
    focus_window(&SharedBus::default(), window, &lease)
}

fn cause(error: &agent_desktop_core::AdapterError) -> String {
    error
        .details
        .as_ref()
        .and_then(|details| details["cause"].as_str())
        .unwrap_or("none")
        .to_owned()
}

fn root_stack() -> Vec<u32> {
    let output = Command::new("xprop")
        .args(["-root", "_NET_CLIENT_LIST_STACKING"])
        .output()
        .expect("xprop runs");
    String::from_utf8_lossy(&output.stdout)
        .split("# ")
        .nth(1)
        .unwrap_or("")
        .split(',')
        .filter_map(|id| u32::from_str_radix(id.trim().trim_start_matches("0x"), 16).ok())
        .collect()
}

fn client_of(pid: u32) -> u32 {
    let started = Instant::now();
    while started.elapsed() < SETTLE {
        if let X11State::Ready { clients, .. } = DisplayReader.clients()
            && let Some(client) = clients.iter().find(|client| client.pid == Some(pid))
        {
            return client.id;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("pid {pid} never had an X11 client");
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/remote-linux.sh"]
fn focus_window_live_stacking_matches_the_root_list_and_sees_an_override_redirect_cover() {
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let mut windows_by_pid = Vec::new();
    for kind in ["gtk3", "gtk4", "qt"] {
        let pid = spawn(&mut launched, kind);
        windows_by_pid.push((pid, wait_for(&bus, pid)));
    }
    let ids: Vec<u32> = windows_by_pid
        .iter()
        .map(|(pid, _)| client_of(*pid))
        .collect();
    let listed = root_stack();
    let lowest = *listed
        .iter()
        .find(|id| ids.contains(id))
        .expect("a launched client is stacked");
    let started = Instant::now();
    let StackingRead::Ready(snapshot) = StackingSource::read(&DisplayReader, lowest) else {
        panic!("the stacking read must be ready under a window manager");
    };
    let position = listed.iter().position(|id| *id == lowest).expect("listed");
    let seen: Vec<u32> = snapshot.stack.iter().map(|window| window.id).collect();
    println!(
        "focus_window_live stacking: {} entries in {}ms, xprop tail {:?}, read {:?}, desktop {:?}",
        seen.len(),
        started.elapsed().as_millis(),
        &listed[position..],
        seen,
        snapshot.current_desktop
    );
    assert_eq!(seen, listed[position..].to_vec());
    assert!(snapshot.stack.iter().all(|window| window.flags.mapped));
    println!(
        "focus_window_live stacking: baseline overlays {:?}",
        snapshot.override_redirect_above
    );
    assert!(
        snapshot
            .override_redirect_above
            .iter()
            .all(|rect| rect.width < 100)
    );

    let cover_pid = launched.spawn("python3", &[&script_path("adf-popup", POPUP)]);
    let started = Instant::now();
    let cover = loop {
        if let StackingRead::Ready(next) = StackingSource::read(&DisplayReader, lowest)
            && let Some(rect) = next
                .override_redirect_above
                .iter()
                .copied()
                .find(|rect| rect.width >= 200)
        {
            break rect;
        }
        assert!(
            started.elapsed() < SETTLE,
            "pid {cover_pid} never mapped its popup"
        );
        std::thread::sleep(Duration::from_millis(200));
    };
    println!("focus_window_live stacking: override-redirect cover {cover:?}");
    assert!(cover.width >= 200 && cover.height >= 100);

    let lowest_window = windows_by_pid
        .iter()
        .find(|(pid, _)| client_of(*pid) == lowest)
        .map(|(_, window)| window.clone())
        .expect("the lowest window");
    if let Err(error) = focus(&lowest_window) {
        panic!(
            "focus_window_live stacking: the raise was refused, so the stack was never reordered: {}",
            cause(&error)
        );
    }
    let raised = root_stack();
    println!(
        "focus_window_live stacking: after activation top is {:?}",
        raised.last()
    );
    assert_eq!(raised.last(), Some(&lowest));
    let StackingRead::Ready(after) = StackingSource::read(&DisplayReader, lowest) else {
        panic!("ready");
    };
    assert_eq!(after.stack.len(), 1);
}
