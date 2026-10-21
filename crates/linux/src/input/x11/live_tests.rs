use std::process::Command;
use std::time::{Duration, Instant};

use agent_desktop_core::Deadline;

use super::session::{live_keymap, run};
use crate::input::keymap::Keymap;
use crate::input::plan::Plan;
use crate::input::step::Step;

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

pub(crate) fn deadline(ms: u64) -> Deadline {
    Deadline::detached_after(ms).expect("deadline")
}

const OWN_DISPLAY_ENV: &str = "AD_OWN_DISPLAY";

pub(crate) fn refuse_live_display(display: &str) {
    let name = display.split('.').next().unwrap_or_default();
    assert!(
        !display.is_empty() && !matches!(name, ":0" | ":1" | ":99"),
        "refusing the display {display:?}: live tests take their display only from a private session"
    );
}

pub(crate) fn private_display() -> String {
    let dir = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION")
        .expect("live input tests run only inside scripts/private-session.sh");
    let current = std::env::var("DISPLAY").unwrap_or_default();
    refuse_live_display(&current);
    let private = match std::env::var(OWN_DISPLAY_ENV) {
        Ok(own) => own,
        Err(_) => {
            let number = std::fs::read_to_string(format!("{dir}/displayfd")).expect("displayfd");
            format!(":{}", number.trim())
        }
    };
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
        let lease = crate::system::interaction_lease::acquire(deadline(70_000)).expect("lease");
        let _ = run(&display, vec![hold], &mut || Ok(()), &lease);
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
    let lease = crate::system::interaction_lease::acquire(deadline(8_000)).expect("lease");
    run(&display, vec![nudge], &mut || Ok(()), &lease).expect("next call");
    assert!(
        !device_state(device).contains(&marker) && !device_state(pointer).contains(button_marker),
        "the next call did not release the input a killed client left held"
    );
}
