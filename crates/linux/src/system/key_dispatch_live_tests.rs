use std::process::Command;
use std::time::{Duration, Instant};

use agent_desktop_core::{
    Deadline, InteractionPolicy, KeyCombo, ProcessId, ProcessIdentity, WindowFilter, WindowInfo,
};

use super::focus_window::focus_window;
use super::interaction_lease::acquire;
use super::key_dispatch::press_key_for_app;
use super::process_facts;
use super::{SharedBus, list_windows};
use crate::tree::live_launch::Launched;

const SETTLE: Duration = Duration::from_secs(40);

const KEYED: &str = r#"
import os, gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk, Gdk
log = os.environ["ADK_LOG"]
def key(widget, event):
    name = Gdk.keyval_name(event.keyval)
    ctrl = bool(event.state & Gdk.ModifierType.CONTROL_MASK)
    with open(log, "a") as out:
        out.write(("ctrl+" if ctrl else "") + name + "\n")
    return False
w = Gtk.Window(title=os.environ["ADK_TITLE"])
w.set_default_size(320, 200)
w.add(Gtk.Label(label="k"))
w.connect("key-press-event", key)
w.connect("destroy", Gtk.main_quit)
w.show_all()
Gtk.main()
"#;

const GRAB: &str = r#"
import os, gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk, Gdk, GLib
log = os.environ["ADK_LOG"]
w = Gtk.Window(title="adk-grab")
w.set_default_size(320, 200)
w.add(Gtk.Label(label="m"))
w.connect("destroy", Gtk.main_quit)
w.show_all()
menu = Gtk.Menu()
for name in ("one", "two"):
    menu.append(Gtk.MenuItem(label=name))
def key(widget, event):
    with open(log, "a") as out:
        out.write(Gdk.keyval_name(event.keyval) + "\n")
    return False
menu.connect("key-press-event", key)
menu.show_all()
def pop():
    menu.popup(None, None, None, None, 0, Gtk.get_current_event_time())
    with open(log + ".state", "w") as out:
        out.write("visible=%s\n" % menu.get_visible())
    return False
GLib.timeout_add(1500, pop)
Gtk.main()
"#;

struct App {
    window: WindowInfo,
    log: String,
}

fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).expect("a deadline builds")
}

fn session_dir() -> String {
    std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("a private session")
}

fn assert_private_display() {
    let display = std::env::var("DISPLAY").expect("DISPLAY");
    let xvfb = std::fs::read_to_string(format!("{}/displayfd", session_dir())).unwrap_or_default();
    assert_eq!(
        display,
        format!(":{}", xvfb.trim()),
        "input is sent only to the private display"
    );
}

fn xprop(args: &[&str]) -> String {
    let output = Command::new("xprop")
        .args(args)
        .output()
        .expect("xprop runs");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn active_pid() -> Option<u32> {
    let active = xprop(&["-root", "_NET_ACTIVE_WINDOW"]);
    let id = active.split_whitespace().last()?.to_owned();
    let owner = xprop(&["-id", &id, "_NET_WM_PID"]);
    owner.split('=').nth(1)?.trim().parse().ok()
}

fn tool(name: &str) -> Command {
    let home = std::env::var("HOME").expect("HOME");
    let mut command = Command::new(format!("{home}/.local/ad-tools/root/usr/bin/{name}"));
    command.env(
        "LD_LIBRARY_PATH",
        format!("{home}/.local/ad-tools/root/usr/lib/x86_64-linux-gnu"),
    );
    command
}

fn xid_of(pid: u32) -> String {
    let clients = xprop(&["-root", "_NET_CLIENT_LIST"]);
    clients
        .split('#')
        .nth(1)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .find(|id| {
            xprop(&["-id", id, "_NET_WM_PID"])
                .split('=')
                .nth(1)
                .and_then(|text| text.trim().parse::<u32>().ok())
                == Some(pid)
        })
        .expect("the display lists the app's window")
        .to_owned()
}

fn close_overview() {
    let status = tool("xdotool")
        .args(["key", "Escape"])
        .status()
        .expect("xdotool runs");
    assert!(status.success());
    std::thread::sleep(Duration::from_millis(2_500));
}

fn windows_of(bus: &SharedBus, pid: u32) -> Vec<WindowInfo> {
    let everything = WindowFilter {
        focused_only: false,
        app: None,
    };
    list_windows(bus, &everything, deadline(10_000))
        .unwrap_or_default()
        .into_iter()
        .filter(|window| window.pid == pid)
        .collect()
}

fn spawn(launched: &mut Launched, bus: &SharedBus, name: &str, source: &str) -> App {
    let dir = session_dir();
    let script = format!("{dir}/adk-{name}.py");
    std::fs::write(&script, source).expect("a script writes");
    let log = format!("{dir}/adk-{name}.log");
    let _ = std::fs::remove_file(&log);
    let pid = launched.spawn_with_env(
        "python3",
        &[&script],
        &[("ADK_LOG", log.clone()), ("ADK_TITLE", name.to_owned())],
    );
    let started = Instant::now();
    while started.elapsed() < SETTLE {
        if let Some(window) = windows_of(bus, pid).into_iter().next() {
            return App { window, log };
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("{name} never listed a window");
}

fn keys_of(app: &App) -> String {
    std::fs::read_to_string(&app.log).unwrap_or_default()
}

fn identity(app: &App) -> ProcessIdentity {
    let pid = app.window.pid.get();
    let facts = process_facts::read(pid).expect("the app's process facts read");
    ProcessIdentity::new(ProcessId::new(pid), facts.instance)
}

fn ctrl_n() -> KeyCombo {
    KeyCombo {
        key: "n".to_owned(),
        modifiers: vec![agent_desktop_core::Modifier::Ctrl],
    }
}

fn press(
    bus: &SharedBus,
    app: &App,
    policy: InteractionPolicy,
) -> Result<(), agent_desktop_core::AdapterError> {
    let lease = acquire(deadline(8_000)).expect("a lease");
    press_key_for_app(bus, identity(app), &ctrl_n(), policy, &lease).map(|_| ())
}

fn click_center(window: &WindowInfo) {
    let bounds = window.bounds.expect("bounds");
    let x = (bounds.x + bounds.width / 2.0) as i64;
    let y = (bounds.y + bounds.height / 2.0) as i64;
    for args in [
        vec!["mousemove".to_owned(), x.to_string(), y.to_string()],
        vec!["click".to_owned(), "1".to_owned()],
    ] {
        let status = tool("xdotool").args(&args).status().expect("xdotool runs");
        assert!(status.success());
        std::thread::sleep(Duration::from_millis(400));
    }
}

fn bring_forward(bus: &SharedBus, app: &App) {
    let lease = acquire(deadline(8_000)).expect("a lease");
    let result = focus_window(bus, &app.window, &lease);
    println!(
        "key_dispatch_live focus {} -> {:?}; active pid {:?}",
        app.window.pid.get(),
        result.as_ref().map_err(|error| error.message.clone()),
        active_pid()
    );
    if result.is_err() {
        click_center(&app.window);
    }
}

fn wait_until(what: &str, done: impl Fn() -> bool) {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(5) {
        if done() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("{what}");
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
fn key_dispatch_live_an_active_target_gets_the_chord_under_either_policy() {
    assert_private_display();
    close_overview();
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let target = spawn(&mut launched, &bus, "target", KEYED);
    let other = spawn(&mut launched, &bus, "other", KEYED);
    std::thread::sleep(Duration::from_secs(1));
    for policy in [InteractionPolicy::headless(), InteractionPolicy::headed()] {
        bring_forward(&bus, &target);
        wait_until("the target never became active", || {
            active_pid() == Some(target.window.pid.get())
        });
        let chords = |app: &App| {
            keys_of(app)
                .lines()
                .filter(|line| *line == "ctrl+n")
                .count()
        };
        let before = chords(&target);
        press(&bus, &target, policy).expect("the active target takes the chord");
        wait_until("the target never logged the chord", || {
            chords(&target) > before
        });
        assert_eq!(chords(&target), before + 1);
        assert_eq!(keys_of(&other), "");
    }
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
fn key_dispatch_live_a_background_target_gets_nothing_under_either_policy() {
    assert_private_display();
    close_overview();
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let target = spawn(&mut launched, &bus, "target", KEYED);
    let front = spawn(&mut launched, &bus, "front", KEYED);
    bring_forward(&bus, &front);
    wait_until("the other window never became active", || {
        active_pid() == Some(front.window.pid.get())
    });
    for policy in [
        InteractionPolicy::headless(),
        InteractionPolicy::focus_fallback(),
    ] {
        let error = press(&bus, &target, policy).expect_err("a background target is refused");
        println!("key_dispatch_live background: {}", error.message);
        std::thread::sleep(Duration::from_millis(500));
        assert_eq!(keys_of(&target), "");
        assert_eq!(keys_of(&front), "");
    }
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
fn key_dispatch_live_a_popup_grab_in_another_app_gets_no_keys_and_the_target_none() {
    assert_private_display();
    close_overview();
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let target = spawn(&mut launched, &bus, "target", KEYED);
    let holder = spawn(&mut launched, &bus, "grab", GRAB);
    std::thread::sleep(Duration::from_secs(3));
    let status = tool("wmctrl")
        .args(["-i", "-a", &xid_of(target.window.pid.get())])
        .status()
        .expect("wmctrl runs");
    assert!(status.success());
    wait_until("the display never named the target active", || {
        active_pid() == Some(target.window.pid.get())
    });
    println!(
        "key_dispatch_live grab state {:?}",
        std::fs::read_to_string(format!("{}.state", holder.log))
    );
    let error = press(&bus, &target, InteractionPolicy::headed())
        .expect_err("a held grab leaves the frame inactive and the chord unsent");
    println!(
        "key_dispatch_live grab: {} {:?}",
        error.message, error.details
    );
    assert_eq!(
        error.details.as_ref().expect("details")["check"],
        "window_active"
    );
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(keys_of(&target), "");
    assert_eq!(keys_of(&holder), "");
}
