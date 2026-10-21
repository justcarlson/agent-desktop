use std::collections::BTreeSet;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use agent_desktop_core::{Deadline, DisplayInfo, MouseButton, Rect};

use super::container::{Anchor, Landmark, locate_anchor};
use super::identity::find_root;
use super::kinds::ShellKind;
use super::open::is_showing;
use crate::input::shell_aim::aim;
use crate::input::shell_click::run_pointer;
use crate::input::wayland::key_session::KeySession;
use crate::input::wayland::remote_desktop::RemoteDesktop;
use crate::system::SharedBus;

const ATTEMPTS: u32 = 8;
const HEALTH_READS: u32 = 5;
const SOCKET_WAIT: Duration = Duration::from_secs(20);

fn deadline() -> Deadline {
    Deadline::after(30_000).expect("a deadline builds")
}

struct HeadlessShell {
    child: Child,
}

fn runtime_dir() -> PathBuf {
    let dir = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION")
        .expect("run inside scripts/private-session.sh: this test starts a shell");
    assert!(
        dir.contains("/rt."),
        "refusing to start a shell outside a private session"
    );
    PathBuf::from(dir)
}

fn sockets() -> BTreeSet<String> {
    std::fs::read_dir(runtime_dir())
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter(|name| name.starts_with("wayland-") && !name.ends_with(".lock"))
                .collect()
        })
        .unwrap_or_default()
}

fn pointer_session_healthy() -> bool {
    for _ in 0..HEALTH_READS {
        let Ok(until) = Deadline::after(3_000) else {
            return false;
        };
        let Ok(mut session) = RemoteDesktop::open(until) else {
            return false;
        };
        let started = session.create(until).and_then(|()| session.start(until));
        let _ = session.stop(until);
        if started.is_err() {
            return false;
        }
        std::thread::sleep(Duration::from_millis(400));
    }
    true
}

fn clear_stale_sockets() {
    for name in sockets() {
        let _ = std::fs::remove_file(runtime_dir().join(&name));
        let _ = std::fs::remove_file(runtime_dir().join(format!("{name}.lock")));
    }
}

fn spawn() -> Option<HeadlessShell> {
    clear_stale_sockets();
    let before = sockets();
    let log = std::fs::File::create(runtime_dir().join("wayland-shell.log")).ok()?;
    let child = Command::new("gnome-shell")
        .args(["--headless", "--wayland", "--virtual-monitor", "1280x800"])
        .env_remove("DISPLAY")
        .stdin(Stdio::null())
        .stdout(log.try_clone().ok()?)
        .stderr(log)
        .process_group(0)
        .spawn()
        .ok()?;
    let shell = HeadlessShell { child };
    let until = Instant::now() + SOCKET_WAIT;
    while Instant::now() < until {
        if sockets().difference(&before).next().is_some() {
            std::thread::sleep(Duration::from_secs(3));
            return Some(shell);
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    None
}

impl HeadlessShell {
    fn start() -> Self {
        for _ in 0..ATTEMPTS {
            if let Some(shell) = spawn() {
                if pointer_session_healthy() {
                    return shell;
                }
            }
        }
        panic!("no healthy headless gnome-shell after {ATTEMPTS} starts");
    }
}

impl Drop for HeadlessShell {
    fn drop(&mut self) {
        let group = format!("-{}", self.child.id());
        let _ = Command::new("kill").args(["-KILL", "--", &group]).status();
        let _ = self.child.kill();
        let _ = self.child.wait();
        std::thread::sleep(Duration::from_secs(2));
    }
}

fn display() -> DisplayInfo {
    DisplayInfo {
        id: "virtual".to_owned(),
        bounds: Rect {
            x: 0.0,
            y: 0.0,
            width: 1280.0,
            height: 800.0,
        },
        is_primary: true,
        scale: 1.0,
    }
}

fn list_showing(bus: &SharedBus) -> bool {
    is_showing(bus, ShellKind::MessageList, deadline()).expect("the list state reads")
}

fn wait_for(bus: &SharedBus, wanted: bool) -> bool {
    let until = Instant::now() + Duration::from_secs(8);
    while Instant::now() < until {
        if list_showing(bus) == wanted {
            return true;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    false
}

#[test]
#[ignore = "starts a headless shell in a private session; run inside scripts/private-session.sh"]
fn a_remote_desktop_pointer_click_opens_and_closes_the_list_on_a_wayland_shell() {
    let _shell = HeadlessShell::start();
    let bus = SharedBus::default();
    let root = find_root(&bus, deadline()).expect("the shell root");
    let date = locate_anchor(
        &bus,
        &root,
        Anchor::Landmark(Landmark::DateMenu),
        deadline(),
    )
    .expect("the date menu is located");
    let extents = date.anchor.placed().expect("it has extents");
    let target = aim(Rect::from(extents), &[display()]).expect("it aims on the display");
    println!("date menu extents {extents:?} target {target:?}");
    let started = list_showing(&bus);
    println!("list started showing={started}");
    for expected in [!started, started] {
        let mut session = RemoteDesktop::open(deadline()).expect("the session bus answers");
        run_pointer(&mut session, &target, &MouseButton::Left, deadline())
            .expect("the pointer route delivers");
        let flipped = wait_for(&bus, expected);
        println!("expected {expected}, flipped {flipped}");
        assert!(flipped, "the list is now {expected}");
    }
}
