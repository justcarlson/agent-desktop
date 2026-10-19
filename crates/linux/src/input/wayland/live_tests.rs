use std::process::Command;
use std::time::{Duration, Instant};

use agent_desktop_core::{AdapterError, Deadline, ErrorCode};

use super::key_session::KeySession;
use super::keysym_send::{run, send_keys};
use super::lead_key::LeadPolicy;
use super::remote_desktop::RemoteDesktop;
use crate::input::step::KeysymStep;
use crate::system::SharedBus;
use crate::system::session_bus::SessionBus;
use crate::tree::live_launch::Launched;

const APP: &str = r#"
import gi
gi.require_version("Gtk", "4.0")
from gi.repository import Gtk

def activate(app):
    window = Gtk.ApplicationWindow(application=app, title="adwl-keys")
    entry = Gtk.Entry()
    window.set_child(entry)
    window.connect("notify::is-active", lambda w, p: (window.set_focus(entry), entry.grab_focus()))
    window.present()
    window.set_focus(entry)

app = Gtk.Application(application_id="org.agentdesktop.WaylandKeys")
app.connect("activate", activate)
app.run([])
"#;

const READER: &str = r#"
import sys
import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi

pid = int(sys.argv[1])

def walk(node, found, depth=0):
    if depth > 12:
        return
    role = node.get_role_name()
    if node.get_state_set().contains(Atspi.StateType.ACTIVE):
        found["active"] = True
    if role in ("text", "entry"):
        try:
            found["text"] = Atspi.Text.get_text(node, 0, -1)
        except Exception:
            pass
    for index in range(node.get_child_count()):
        child = node.get_child_at_index(index)
        if child is not None:
            walk(child, found, depth + 1)

desktop = Atspi.get_desktop(0)
found = {}
for index in range(desktop.get_child_count()):
    app = desktop.get_child_at_index(index)
    if app is not None and app.get_process_id() == pid:
        walk(app, found)
print("active=%d" % (1 if found.get("active") else 0))
print("text=" + found.get("text", "<none>"))
"#;

struct Read {
    active: bool,
    text: String,
}

fn read(pid: u32) -> Option<Read> {
    let output = Command::new("python3")
        .args(["-I", "-c", READER, &pid.to_string()])
        .output()
        .ok()?;
    let stdout = String::from_utf8(output.stdout).ok()?;
    let mut lines = stdout.lines();
    let active = lines.next()? == "active=1";
    let text = lines.next()?.strip_prefix("text=")?.to_owned();
    Some(Read { active, text })
}

fn wait_for(pid: u32, accept: impl Fn(&Read) -> bool) -> Option<Read> {
    let until = Instant::now() + Duration::from_secs(15);
    while Instant::now() < until {
        if let Some(now) = read(pid).filter(&accept) {
            return Some(now);
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    None
}

fn launch(launched: &mut Launched) -> u32 {
    let display = std::env::var("WAYLAND_DISPLAY").expect("WAYLAND_DISPLAY names the shell socket");
    let pid = launched.spawn_with_env(
        "python3",
        &["-I", "-c", APP],
        &[
            ("GDK_BACKEND", "wayland".into()),
            ("WAYLAND_DISPLAY", display),
            ("GTK_USE_PORTAL", "0".into()),
            ("GTK_IM_MODULE", "gtk-im-context-simple".into()),
            ("GDK_DEBUG", "no-portals".into()),
            ("DISPLAY", String::new()),
        ],
    );
    wait_for(pid, |_| true).expect("the entry is on the accessibility bus");
    pid
}

fn keys(text: &str) -> Vec<KeysymStep> {
    text.chars()
        .flat_map(|c| {
            let keysym = if c.is_ascii() {
                u32::from(c)
            } else {
                0x0100_0000 | u32::from(c)
            };
            [
                KeysymStep { keysym, down: true },
                KeysymStep {
                    keysym,
                    down: false,
                },
            ]
        })
        .collect()
}

fn close_overview() {
    let none = LeadPolicy {
        tap: None,
        settle: Duration::ZERO,
    };
    let escape = [
        KeysymStep {
            keysym: 0xff1b,
            down: true,
        },
        KeysymStep {
            keysym: 0xff1b,
            down: false,
        },
    ];
    let mut session = RemoteDesktop::open(deadline()).expect("the session bus opens");
    run(&mut session, none, &escape, &mut || Ok(()), deadline()).expect("Escape delivers");
    std::thread::sleep(Duration::from_millis(1500));
}

fn deadline() -> Deadline {
    Deadline::after(20_000).expect("a deadline builds")
}

fn send(pid: u32, steps: &[KeysymStep]) -> Result<u32, AdapterError> {
    let mut gate = || {
        wait_for(pid, |now| now.active)
            .map(|_| ())
            .ok_or_else(|| AdapterError::internal("the frame never became active"))
    };
    send_keys(steps, &mut gate, deadline()).map(|delivery| delivery.delivered_events)
}

#[test]
#[ignore = "needs a private Wayland session with a headless gnome-shell; run through scripts/private-session.sh"]
fn remote_desktop_answers_on_the_session_bus_and_not_on_the_accessibility_bus() {
    let mut desktop = RemoteDesktop::open(deadline()).expect("the session bus opens");
    desktop.create(deadline()).expect("CreateSession succeeds");
    desktop.start(deadline()).expect("Start succeeds");
    desktop.stop(deadline()).expect("Stop succeeds");

    let accessibility = SharedBus::default()
        .get(deadline())
        .expect("the accessibility bus opens");
    let mut wrong = RemoteDesktop::over(accessibility.inner().clone());
    let error = wrong
        .create(deadline())
        .expect_err("the accessibility bus has no RemoteDesktop");
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    assert!(SessionBus::get(deadline()).is_ok());
}

#[test]
#[ignore = "needs a private Wayland session with a headless gnome-shell; run through scripts/private-session.sh"]
fn keys_land_in_a_wayland_native_gtk4_entry() {
    let mut launched = Launched::new();
    let pid = launch(&mut launched);
    close_overview();

    send(pid, &keys("hello")).expect("the keys deliver");
    let typed = wait_for(pid, |now| now.text == "hello").map(|now| now.text);
    let last = read(pid).map(|now| (now.active, now.text));
    assert_eq!(typed.as_deref(), Some("hello"), "last read {last:?}");

    let mut select_all = vec![
        KeysymStep {
            keysym: 0xffe3,
            down: true,
        },
        KeysymStep {
            keysym: 0x61,
            down: true,
        },
        KeysymStep {
            keysym: 0x61,
            down: false,
        },
        KeysymStep {
            keysym: 0xffe3,
            down: false,
        },
    ];
    select_all.extend(keys("x"));
    send(pid, &select_all).expect("ctrl+a then x deliver");
    let replaced = wait_for(pid, |now| now.text == "x").map(|now| now.text);
    assert_eq!(replaced.as_deref(), Some("x"));

    send(pid, &keys("你好")).expect("wide characters deliver");
    let wide = wait_for(pid, |now| now.text == "x你好").map(|now| now.text);
    assert_eq!(wide.as_deref(), Some("x你好"));
}
