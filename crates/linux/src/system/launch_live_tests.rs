use std::collections::BTreeMap;
use std::process::Command;
use std::time::{Duration, Instant};

use agent_desktop_core::launch_options::LaunchOptions;
use agent_desktop_core::launch_result::LaunchResult;
use agent_desktop_core::{AdapterError, Deadline, ErrorCode};

use super::interaction_lease::acquire;
use super::launch::launch_app;
use super::process_facts;
use super::{SharedBus, app_inventory};
use crate::tree::live_launch::nonce;

const SETTLE: Duration = Duration::from_secs(20);

fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).expect("a deadline builds")
}

fn options(args: &[&str]) -> LaunchOptions {
    LaunchOptions {
        args: args.iter().map(|arg| (*arg).to_owned()).collect(),
        timeout_ms: 15_000,
        ..LaunchOptions::default()
    }
}

fn launch(id: &str, options: &LaunchOptions) -> Result<LaunchResult, AdapterError> {
    assert!(
        std::env::var_os("AGENT_DESKTOP_PRIVATE_SESSION").is_some(),
        "live launches must run inside scripts/private-session.sh"
    );
    let lease = acquire(deadline(30_000)).expect("a lease");
    launch_app(&SharedBus::default(), id, options, &lease)
}

struct Owned(Vec<u32>);

impl Owned {
    fn keep(&mut self, pid: u32) -> u32 {
        self.0.push(pid);
        pid
    }
}

impl Drop for Owned {
    fn drop(&mut self) {
        for pid in &self.0 {
            kill_quietly(*pid);
        }
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(5)
            && self.0.iter().any(|pid| process_facts::read(*pid).is_some())
        {
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

fn kill_quietly(pid: u32) {
    let _ = Command::new("kill")
        .args(["-KILL", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status();
}

fn xprop(args: &[&str]) -> String {
    let output = Command::new("xprop")
        .args(args)
        .output()
        .expect("xprop runs");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn client_ids() -> Vec<String> {
    xprop(&["-root", "_NET_CLIENT_LIST"])
        .split('#')
        .nth(1)
        .unwrap_or_default()
        .split(',')
        .map(|id| id.trim().to_owned())
        .filter(|id| id.starts_with("0x"))
        .collect()
}

fn x_windows_of(pid: u32) -> Vec<String> {
    client_ids()
        .into_iter()
        .filter(|id| {
            xprop(&["-id", id, "_NET_WM_PID"])
                .rsplit('=')
                .next()
                .and_then(|value| value.trim().parse::<u32>().ok())
                == Some(pid)
        })
        .collect()
}

fn wait_until(what: &str, mut condition: impl FnMut() -> bool) {
    let started = Instant::now();
    while started.elapsed() < SETTLE {
        if condition() {
            return;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("timed out waiting for {what}");
}

fn registry_has(pid: u32) -> bool {
    app_inventory(&SharedBus::default(), deadline(10_000))
        .is_ok_and(|inventory| inventory.apps.iter().any(|app| app.pid == pid))
}

fn assert_owner_is_truthful(result: &LaunchResult) {
    let pid = result.pid.get();
    wait_until("the owner on the registry", || registry_has(pid));
    wait_until("an X11 window of the owner", || {
        !x_windows_of(pid).is_empty()
    });
    let facts = process_facts::read(pid).expect("the owner runs");
    assert_eq!(
        result.process_instance.as_deref(),
        Some(facts.instance.as_str())
    );
    let window = result.window.as_ref().expect("a window inside the budget");
    assert_eq!(window.pid, result.pid);
}

fn kill_and_wait(pid: u32) {
    kill_quietly(pid);
    wait_until("the process to go", || process_facts::read(pid).is_none());
}

fn launched(owned: &mut Owned, id: &str, args: &[&str]) -> LaunchResult {
    let result = launch(id, &options(args)).unwrap_or_else(|error| panic!("{id}: {error:?}"));
    owned.keep(result.pid.get());
    result
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_calculator_launches_by_command_id_path_and_entry_name() {
    let mut owned = Owned(Vec::new());
    for id in [
        "gnome-calculator",
        "org.gnome.Calculator",
        "/usr/bin/gnome-calculator",
        "Calculator",
    ] {
        let result = launched(&mut owned, id, &[]);
        assert_owner_is_truthful(&result);
        assert_eq!(result.app, "gnome-calculator", "{id}");
        kill_and_wait(result.pid.get());
    }
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_a_second_calculator_attaches_without_a_spawn_and_with_arguments_is_a_new_owner() {
    let mut owned = Owned(Vec::new());
    let first = launched(&mut owned, "gnome-calculator", &[]);
    assert_owner_is_truthful(&first);
    let again = launched(&mut owned, "Calculator", &[]);
    assert_eq!(again.pid, first.pid);
    let refused = launch(
        "Calculator",
        &LaunchOptions {
            attach_if_running: false,
            ..options(&[])
        },
    )
    .expect_err("a fresh launch was required");
    assert_eq!(refused.code, ErrorCode::ActionFailed);
    assert!(refused.message.contains(&first.pid.get().to_string()));
    let second = launched(&mut owned, "Calculator", &["--mode=basic"]);
    assert_ne!(second.pid, first.pid);
    assert_owner_is_truthful(&second);
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_text_editor_by_desktop_id_then_a_file_reaches_the_first_instance() {
    let mut owned = Owned(Vec::new());
    let first = launched(&mut owned, "org.gnome.TextEditor", &[]);
    assert_owner_is_truthful(&first);
    let dir = std::env::temp_dir();
    let file = dir.join(format!("{}.txt", nonce("launch-doc")));
    std::fs::write(&file, "hello").expect("a document");
    let name = file
        .file_name()
        .expect("name")
        .to_string_lossy()
        .into_owned();
    let with_file = LaunchOptions {
        env: BTreeMap::from([("LAUNCH_LIVE_MARKER".to_owned(), "1".to_owned())]),
        ..options(&[&file.to_string_lossy()])
    };
    let again = launch("org.gnome.TextEditor", &with_file).expect("the forwarding launch");
    assert_eq!(again.pid, first.pid);
    assert!(
        again
            .suggestion
            .as_deref()
            .is_some_and(|text| text.contains("not applied"))
    );
    wait_until("the document in a window of the first pid", || {
        x_windows_of(first.pid.get())
            .iter()
            .any(|id| xprop(&["-id", id, "_NET_WM_NAME"]).contains(&name))
    });
    let editors = client_ids()
        .iter()
        .filter(|id| {
            let class = xprop(&["-id", id, "WM_CLASS"]).to_lowercase();
            class.contains("text-editor") || class.contains("texteditor")
        })
        .map(|id| xprop(&["-id", id, "_NET_WM_PID"]))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(editors.len(), 1, "one application process: {editors:?}");
    assert!(
        editors
            .iter()
            .all(|pid| pid.contains(&first.pid.get().to_string()))
    );
    let _ = std::fs::remove_file(file);
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_a_qt_app_launches_by_path_with_the_environment_given() {
    let home = std::env::var("HOME").expect("HOME");
    let dir = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("a private session");
    let script = format!("{dir}/launch-qt.py");
    std::fs::write(
        &script,
        "import sys\nfrom PyQt6.QtWidgets import QApplication, QMainWindow\napp = QApplication(sys.argv)\nw = QMainWindow()\nw.setWindowTitle('adl-qt')\nw.resize(300, 200)\nw.show()\nsys.exit(app.exec())\n",
    )
    .expect("a script");
    let python = format!("{home}/.local/ad-tools/qtvenv/bin/python");
    let libraries = format!("{home}/.local/ad-tools/root/usr/lib/x86_64-linux-gnu");
    let launch_options = LaunchOptions {
        env: BTreeMap::from([
            ("QT_ACCESSIBILITY".to_owned(), "1".to_owned()),
            (
                "QT_LINUX_ACCESSIBILITY_ALWAYS_ON".to_owned(),
                "1".to_owned(),
            ),
            ("QT_QPA_PLATFORM".to_owned(), "xcb".to_owned()),
            ("LD_LIBRARY_PATH".to_owned(), libraries),
        ]),
        ..options(&[&script])
    };
    let mut owned = Owned(Vec::new());
    let result = launch(&python, &launch_options).expect("the Qt launch");
    owned.keep(result.pid.get());
    assert_owner_is_truthful(&result);
    let environ = std::fs::read(format!("/proc/{}/environ", result.pid.get())).expect("environ");
    for name in [
        "DISPLAY",
        "DBUS_SESSION_BUS_ADDRESS",
        "XDG_RUNTIME_DIR",
        "QT_ACCESSIBILITY",
    ] {
        let prefix = format!("{name}=");
        assert!(
            environ
                .split(|byte| *byte == 0)
                .any(|var| var.starts_with(prefix.as_bytes())),
            "{name} reached the app"
        );
    }
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_terminal_is_attributed_to_its_server_and_a_second_launch_adds_a_window() {
    let mut owned = Owned(Vec::new());
    let first = launched(&mut owned, "gnome-terminal", &[]);
    let comm = std::fs::read_to_string(format!("/proc/{}/comm", first.pid.get())).expect("comm");
    assert!(
        comm.starts_with("gnome-terminal-"),
        "the owner is the server: {comm}"
    );
    assert_owner_is_truthful(&first);
    let before = x_windows_of(first.pid.get()).len();
    let second = launched(&mut owned, "gnome-terminal", &["--window"]);
    assert_eq!(second.pid, first.pid);
    wait_until("a second terminal window", || {
        x_windows_of(first.pid.get()).len() > before
    });
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_a_missing_name_is_app_not_found_and_nothing_started() {
    let error = launch("no-such-application-xyz", &options(&[])).expect_err("not found");
    assert_eq!(error.code, ErrorCode::AppNotFound);
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_an_app_that_exits_after_registering_and_before_a_window_is_not_a_launch() {
    let dir = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("a private session");
    let script = format!("{dir}/launch-early-exit.py");
    std::fs::write(
        &script,
        "import gi, time\ngi.require_version('Gtk', '3.0')\nfrom gi.repository import Gtk, GLib\nGtk.init()\nend = time.time() + 1.5\nwhile time.time() < end:\n    GLib.MainContext.default().iteration(False)\n    time.sleep(0.05)\n",
    )
    .expect("a script");
    let started = Instant::now();
    let error = launch("python3", &options(&[&script])).expect_err("a dead pid is not a launch");
    assert_eq!(error.code, ErrorCode::ActionFailed, "{error:?}");
    assert!(error.message.contains("exited before"), "{error:?}");
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "the exit is noticed, not waited out: {:?}",
        started.elapsed()
    );
}

#[path = "launch_resident_live_tests.rs"]
mod resident;
