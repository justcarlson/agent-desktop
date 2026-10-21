use std::process::Command;
use std::time::Duration;

use super::super::live_rig::wait_until;

/// The user-directory tools the live runs use: a window-manager client and a
/// pointer-free window mover. They are the oracle, never the code under test.
fn run(program: &str, args: &[&str]) -> String {
    let home = std::env::var("HOME").expect("HOME is set");
    let root = format!("{home}/.local/ad-tools/root");
    let path = format!(
        "{root}/usr/bin:{}",
        std::env::var("PATH").unwrap_or_default()
    );
    let libs = format!("{root}/usr/lib/x86_64-linux-gnu");
    let output = Command::new(program)
        .args(args)
        .env("PATH", path)
        .env("LD_LIBRARY_PATH", libs)
        .output()
        .unwrap_or_else(|error| panic!("{program} did not run: {error}"));
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Geometry {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) width: i32,
    pub(super) height: i32,
}

#[derive(Debug, Clone)]
pub(super) struct XWindow {
    pub(super) id: u32,
    pub(super) pid: u32,
    pub(super) title: String,
}

pub(super) fn windows() -> Vec<XWindow> {
    run("wmctrl", &["-lp"])
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let id = u32::from_str_radix(parts.next()?.trim_start_matches("0x"), 16).ok()?;
            parts.next()?;
            let pid = parts.next()?.parse().ok()?;
            parts.next()?;
            Some(XWindow {
                id,
                pid,
                title: parts.collect::<Vec<_>>().join(" "),
            })
        })
        .collect()
}

/// The X window of an application's frame: its process, then its title.
pub(super) fn window_of(pid: u32, title: &str) -> Option<XWindow> {
    let mut found = None;
    wait_until(Duration::from_secs(8), || {
        let owned: Vec<XWindow> = windows().into_iter().filter(|w| w.pid == pid).collect();
        found = owned
            .iter()
            .find(|window| window.title == title)
            .or_else(|| owned.iter().max_by_key(|window| window.id))
            .cloned();
        found.is_some()
    });
    found
}

/// The client area in root coordinates, from `xwininfo`.
pub(super) fn geometry(id: u32) -> Option<Geometry> {
    let text = run("xwininfo", &["-id", &id.to_string()]);
    let number = |key: &str| {
        text.lines()
            .find_map(|line| line.trim().strip_prefix(key))
            .and_then(|rest| rest.trim().parse::<i32>().ok())
    };
    Some(Geometry {
        x: number("Absolute upper-left X:")?,
        y: number("Absolute upper-left Y:")?,
        width: number("Width:")?,
        height: number("Height:")?,
    })
}

pub(super) fn set_above(id: u32, on: bool) {
    let change = if on { "add,above" } else { "remove,above" };
    run("wmctrl", &["-i", "-r", &format!("0x{id:x}"), "-b", change]);
}

pub(super) fn raise(id: u32) {
    run("wmctrl", &["-i", "-a", &format!("0x{id:x}")]);
}

pub(super) fn lower(id: u32) {
    run("xdotool", &["windowlower", &id.to_string()]);
}

pub(super) fn minimize(id: u32) {
    run("xdotool", &["windowminimize", &id.to_string()]);
}

/// The uniquely titled fixture window, even when the server reuses a former XID.
pub(super) fn appeared(title: &str, start: impl FnOnce()) -> Option<u32> {
    start();
    let mut found = None;
    wait_until(Duration::from_secs(8), || {
        found = windows()
            .into_iter()
            .find(|window| window.title == title)
            .map(|window| window.id);
        found.is_some()
    });
    found
}
