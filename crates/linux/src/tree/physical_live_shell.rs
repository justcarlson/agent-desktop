use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use super::physical_live_x11::overview_active;
use crate::input::x11::live_tests::refuse_live_display;

const FIRST_DISPLAY: u32 = 100;
const LAST_DISPLAY: u32 = 199;
const ATTEMPTS: u32 = 5;
const XVFB_WAIT: Duration = Duration::from_secs(10);
const SHELL_WAIT: Duration = Duration::from_secs(20);
const POLL: Duration = Duration::from_millis(200);

pub(super) struct X11Shell {
    xvfb: Child,
    shell: Option<Child>,
    pub(super) display: String,
}

fn session_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AGENT_DESKTOP_PRIVATE_SESSION")
            .expect("the shell starts only inside scripts/private-session.sh"),
    )
}

fn display_is_free(number: u32) -> bool {
    !std::path::Path::new(&format!("/tmp/.X11-unix/X{number}")).exists()
        && !std::path::Path::new(&format!("/tmp/.X{number}-lock")).exists()
}

fn start_xvfb() -> (Child, String) {
    let number = (FIRST_DISPLAY..=LAST_DISPLAY)
        .find(|number| display_is_free(*number))
        .expect("a free display number");
    let display = format!(":{number}");
    refuse_live_display(&display);
    let log = std::fs::File::create(session_dir().join("xvfb-shell.log")).expect("xvfb log");
    let child = Command::new("Xvfb")
        .args([&display, "-screen", "0", "1280x800x24", "-nolisten", "tcp"])
        .env_remove("WAYLAND_DISPLAY")
        .stdin(Stdio::null())
        .stdout(log.try_clone().expect("log"))
        .stderr(log)
        .process_group(0)
        .spawn()
        .expect("Xvfb starts");
    let socket = format!("/tmp/.X11-unix/X{number}");
    let until = Instant::now() + XVFB_WAIT;
    while !std::path::Path::new(&socket).exists() {
        assert!(Instant::now() < until, "Xvfb {display} never listened");
        std::thread::sleep(POLL);
    }
    (child, display)
}

fn spawn_shell(display: &str) -> Child {
    let log = std::fs::File::create(session_dir().join("gnome-shell-x11.log")).expect("log");
    Command::new("gnome-shell")
        .args(["--x11", "--replace"])
        .env("DISPLAY", display)
        .env_remove("WAYLAND_DISPLAY")
        .stdin(Stdio::null())
        .stdout(log.try_clone().expect("log"))
        .stderr(log)
        .process_group(0)
        .spawn()
        .expect("gnome-shell starts")
}

fn stop(child: &mut Child) {
    let group = format!("-{}", child.id());
    let _ = Command::new("kill").args(["-KILL", "--", &group]).status();
    let _ = child.kill();
    let _ = child.wait();
}

fn healthy() -> bool {
    let until = Instant::now() + SHELL_WAIT;
    while Instant::now() < until {
        if overview_active().is_ok() {
            return true;
        }
        std::thread::sleep(POLL);
    }
    false
}

impl X11Shell {
    pub(super) fn start() -> Self {
        let (xvfb, display) = start_xvfb();
        let mut holder = Self {
            xvfb,
            shell: None,
            display,
        };
        for _ in 0..ATTEMPTS {
            let mut shell = spawn_shell(&holder.display);
            if healthy() {
                holder.shell = Some(shell);
                return holder;
            }
            stop(&mut shell);
        }
        panic!("no healthy gnome-shell --x11 after {ATTEMPTS} starts");
    }
}

impl Drop for X11Shell {
    fn drop(&mut self) {
        if let Some(shell) = self.shell.as_mut() {
            stop(shell);
        }
        stop(&mut self.xvfb);
        let number = self.display.trim_start_matches(':');
        let _ = std::fs::remove_file(format!("/tmp/.X11-unix/X{number}"));
        let _ = std::fs::remove_file(format!("/tmp/.X{number}-lock"));
    }
}
