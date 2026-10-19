use std::process::Command;
use std::time::{Duration, Instant};

use agent_desktop_core::{AdapterError, Deadline};

use super::session::{live_keymap, run};
use crate::input::keymap::Keymap;
use crate::input::plan::Plan;
use crate::input::step::Step;
use crate::tree::live_launch::Launched;

const SETTLE: Duration = Duration::from_secs(30);

pub(crate) const GTK3: &str = r#"
import gi, os
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk
log = os.environ["AD_LIVE_LOG"]
w = Gtk.Window(title="adx")
w.set_default_size(320, 80)
e = Gtk.Entry()
e.connect("changed", lambda x: open(log, "w").write(x.get_text()))
w.add(e)
w.connect("destroy", Gtk.main_quit)
w.show_all()
Gtk.main()
"#;

pub(crate) const GTK4: &str = r#"
import gi, os
gi.require_version("Gtk", "4.0")
from gi.repository import Gtk, GLib
Gtk.init()
log = os.environ["AD_LIVE_LOG"]
loop = GLib.MainLoop()
w = Gtk.Window(title="adx")
w.set_default_size(320, 80)
e = Gtk.Entry()
e.connect("changed", lambda x: open(log, "w").write(x.get_text()))
w.set_child(e)
w.connect("close-request", lambda *a: loop.quit())
w.present()
loop.run()
"#;

pub(crate) const QT: &str = r#"
import os, sys
from PyQt6.QtWidgets import QApplication, QLineEdit
log = os.environ["AD_LIVE_LOG"]
app = QApplication(sys.argv)
e = QLineEdit()
e.setWindowTitle("adx")
e.textChanged.connect(lambda t: open(log, "w").write(t))
e.resize(320, 80)
e.show()
sys.exit(app.exec())
"#;

pub(crate) fn deadline(ms: u64) -> Deadline {
    Deadline::detached_after(ms).expect("deadline")
}

pub(crate) fn private_display() -> String {
    let dir = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION")
        .expect("live input tests run only inside scripts/private-session.sh");
    let number = std::fs::read_to_string(format!("{dir}/displayfd")).expect("displayfd");
    let private = format!(":{}", number.trim());
    let current = std::env::var("DISPLAY").expect("DISPLAY");
    assert_eq!(
        current, private,
        "input is sent only to the private session's display"
    );
    current
}

fn keycode_for(map: &Keymap, keysym: u32) -> u8 {
    let width = usize::from(map.keysyms_per_keycode);
    let index = map
        .keysyms
        .chunks(width)
        .position(|row| row.first() == Some(&keysym))
        .unwrap_or_else(|| panic!("keysym {keysym:#x} is not on level one"));
    map.min_keycode + u8::try_from(index).expect("keycode")
}

fn type_plan(map: &Keymap, text: &str) -> Plan {
    let mut steps = Vec::new();
    for character in text.chars() {
        let code = keycode_for(map, u32::from(character));
        steps.push(Step::Key { code, down: true });
        steps.push(Step::Key { code, down: false });
    }
    Plan {
        steps,
        release_origin: None,
    }
}

fn click_plan(x: i32, y: i32) -> Plan {
    Plan {
        steps: vec![
            Step::MoveTo { x, y },
            Step::Button {
                code: 1,
                down: true,
            },
            Step::Pause(Duration::from_millis(50)),
            Step::Button {
                code: 1,
                down: false,
            },
        ],
        release_origin: Some((x, y)),
    }
}

pub(crate) fn window_center(name: &str) -> (i32, i32) {
    let started = Instant::now();
    while started.elapsed() < SETTLE {
        let output = Command::new("xwininfo").args(["-name", name]).output();
        if let Ok(output) = output {
            let text = String::from_utf8_lossy(&output.stdout);
            let field = |label: &str| {
                text.lines()
                    .find_map(|line| line.trim().strip_prefix(label))
                    .and_then(|rest| rest.trim().parse::<i32>().ok())
            };
            if let (Some(x), Some(y), Some(w), Some(h)) = (
                field("Absolute upper-left X:"),
                field("Absolute upper-left Y:"),
                field("Width:"),
                field("Height:"),
            ) {
                return (x + w / 2, y + h / 2);
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("window {name} never appeared");
}

pub(crate) fn wait_for_log(path: &str, expected: &str) -> String {
    let started = Instant::now();
    let mut seen = String::new();
    while started.elapsed() < Duration::from_secs(8) {
        seen = std::fs::read_to_string(path).unwrap_or_default();
        if seen == expected {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    seen
}

pub(crate) fn spawn_app(launched: &mut Launched, kind: &str, log: &str) {
    let dir = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("session");
    let script = format!("{dir}/adx-{kind}.py");
    let source = match kind {
        "gtk3" => GTK3,
        "gtk4" => GTK4,
        _ => QT,
    };
    std::fs::write(&script, source).expect("script");
    let log = ("AD_LIVE_LOG", log.to_owned());
    if kind == "qt" {
        let home = std::env::var("HOME").expect("HOME");
        let python = format!("{home}/.local/ad-tools/qtvenv/bin/python");
        let libraries = format!("{home}/.local/ad-tools/root/usr/lib/x86_64-linux-gnu");
        launched.spawn_with_env(
            &python,
            &[&script],
            &[
                log,
                ("QT_QPA_PLATFORM", "xcb".to_owned()),
                ("LD_LIBRARY_PATH", libraries),
            ],
        );
    } else {
        launched.spawn_with_env(
            "python3",
            &[&script],
            &[log, ("PYTHONUTF8", "1".to_owned())],
        );
    }
}

fn type_into(kind: &str) {
    let display = private_display();
    let dir = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("session");
    let log = format!("{dir}/adx-{kind}.log");
    let _ = std::fs::remove_file(&log);
    let mut launched = Launched::new();
    spawn_app(&mut launched, kind, &log);
    let (x, y) = window_center("adx");
    std::thread::sleep(Duration::from_millis(800));
    let mut gate = || -> Result<(), AdapterError> { Ok(()) };
    run(&display, vec![click_plan(x, y)], &mut gate, deadline(8_000)).expect("click");
    let map = live_keymap(&display, deadline(8_000)).expect("keymap");
    let outcome = run(
        &display,
        vec![type_plan(&map, "hello")],
        &mut gate,
        deadline(8_000),
    );
    let seen = wait_for_log(&log, "hello");
    launched.stop();
    let delivery = outcome.expect("typed");
    assert_eq!(delivery.delivered_events, 10);
    assert_eq!(seen, "hello", "the {kind} entry did not receive the keys");
}

#[test]
#[ignore = "needs scripts/private-session.sh --wm"]
fn live_typing_reaches_a_gtk3_entry() {
    type_into("gtk3");
}

#[test]
#[ignore = "needs scripts/private-session.sh --wm"]
fn live_typing_reaches_a_gtk4_entry() {
    type_into("gtk4");
}

#[test]
#[ignore = "needs scripts/private-session.sh --wm"]
fn live_typing_reaches_a_qt_entry() {
    type_into("qt");
}

pub(crate) fn device_state(device: &str) -> String {
    let output = Command::new("xinput")
        .args(["query-state", device])
        .output()
        .expect("xinput runs");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
#[ignore = "needs scripts/private-session.sh --wm"]
fn live_a_key_left_held_by_a_killed_client_is_released_by_the_next_call() {
    let display = private_display();
    if std::env::var("AD_HOLD_HELPER").is_ok() {
        let map = live_keymap(&display, deadline(8_000)).expect("keymap");
        let code = keycode_for(&map, u32::from('a'));
        let hold = Plan {
            steps: vec![
                Step::Key { code, down: true },
                Step::Button {
                    code: 1,
                    down: true,
                },
                Step::Pause(Duration::from_secs(60)),
            ],
            release_origin: None,
        };
        let _ = run(&display, vec![hold], &mut || Ok(()), deadline(70_000));
        return;
    }
    let map = live_keymap(&display, deadline(8_000)).expect("keymap");
    let code = keycode_for(&map, u32::from('a'));
    let device = "Virtual core XTEST keyboard";
    let mut child = Command::new(std::env::current_exe().expect("exe"))
        .args([
            "--exact",
            "input::x11::live_tests::live_a_key_left_held_by_a_killed_client_is_released_by_the_next_call",
            "--include-ignored",
            "--nocapture",
        ])
        .env("AD_HOLD_HELPER", "1")
        .spawn()
        .expect("helper");
    let marker = format!("key[{code}]=down");
    let pointer = "Virtual core XTEST pointer";
    let button_marker = "button[1]=down";
    let started = Instant::now();
    while !(device_state(device).contains(&marker) && device_state(pointer).contains(button_marker))
    {
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "key never went down"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    let pid = child.id();
    Command::new("kill")
        .args(["-KILL", &pid.to_string()])
        .status()
        .expect("kill");
    let _ = child.wait();
    std::thread::sleep(Duration::from_millis(500));
    assert!(
        device_state(device).contains(&marker) && device_state(pointer).contains(button_marker),
        "the server released the input by itself, so the journal is not needed"
    );
    let nudge = Plan {
        steps: vec![Step::MoveTo { x: 3, y: 3 }],
        release_origin: None,
    };
    run(&display, vec![nudge], &mut || Ok(()), deadline(8_000)).expect("next call");
    assert!(
        !device_state(device).contains(&marker) && !device_state(pointer).contains(button_marker),
        "the next call did not release the input a killed client left held"
    );
}
