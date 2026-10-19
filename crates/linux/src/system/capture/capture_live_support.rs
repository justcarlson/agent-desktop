use std::io::Cursor;
use std::process::Command;
use std::time::{Duration, Instant};

use agent_desktop_core::{
    AdapterError, Deadline, DisplayInfo, ImageBuffer, WindowFilter, WindowInfo,
};

use super::window_context::WindowLookup;
use super::window_xid::XidOutcome;

use crate::system::x11::grab::{DisplayGrabber, Grabber};
use crate::system::{SharedBus, list_windows};
use crate::tree::live_launch::Launched;

const SETTLE: Duration = Duration::from_secs(40);

const PAINTER: &str = r##"
import sys, gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk, Gdk, GLib
title, color, x, y, w, h, popup = sys.argv[1:8]
kind = Gtk.WindowType.POPUP if popup == "1" else Gtk.WindowType.TOPLEVEL
win = Gtk.Window(type=kind)
win.set_title(title)
win.set_default_size(int(w), int(h))
css = Gtk.CssProvider()
css.load_from_data(("window, window.background { background-color: %s; background-image: none; }" % color).encode())
Gtk.StyleContext.add_provider_for_screen(Gdk.Screen.get_default(), css, Gtk.STYLE_PROVIDER_PRIORITY_USER)
win.move(int(x), int(y))
win.connect("destroy", Gtk.main_quit)
win.show_all()
GLib.timeout_add(500, lambda: (win.move(int(x), int(y)), False)[1])
Gtk.main()
"##;

const QT_PAINTER: &str = r##"
import sys
from PyQt6.QtGui import QColor, QPalette
from PyQt6.QtWidgets import QApplication, QMainWindow, QWidget
title, color, x, y, w, h = sys.argv[1:7]
app = QApplication(sys.argv)
window = QMainWindow()
window.setWindowTitle(title)
body = QWidget()
body.setAutoFillBackground(True)
palette = body.palette()
palette.setColor(QPalette.ColorRole.Window, QColor(color))
body.setPalette(palette)
window.setCentralWidget(body)
window.resize(int(w), int(h))
window.move(int(x), int(y))
window.show()
sys.exit(app.exec())
"##;

pub(super) const RED: [u8; 3] = [0xff, 0x00, 0x00];
pub(super) const BLUE: [u8; 3] = [0x00, 0x00, 0xff];
pub(super) const BLACK: [u8; 3] = [0x00, 0x00, 0x00];

pub(super) fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).expect("a deadline builds")
}

pub(super) fn assert_private_display() -> crate::system::x11::live_display::Exclusive {
    let guard = crate::system::x11::live_display::exclusive();
    let display = std::env::var("DISPLAY").expect("a private display");
    assert!(
        !matches!(display.as_str(), ":0" | ":99" | ""),
        "the live display {display} must never be captured"
    );
    assert!(
        std::env::var_os("AGENT_DESKTOP_PRIVATE_SESSION").is_some(),
        "run through scripts/private-session.sh"
    );
    guard
}

pub(super) fn compositing() -> bool {
    DisplayGrabber
        .compositing(deadline(10_000))
        .expect("the compositing probe answers")
}

pub(super) struct Geometry {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) width: i32,
    pub(super) height: i32,
}

pub(super) fn paint(
    launched: &mut Launched,
    title: &str,
    color: &str,
    at: &Geometry,
    popup: bool,
) -> u32 {
    let dir = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("a private session");
    let script = format!("{dir}/capture-painter.py");
    std::fs::write(&script, PAINTER).expect("the painter script writes");
    let args = [
        script.as_str(),
        title,
        color,
        &at.x.to_string(),
        &at.y.to_string(),
        &at.width.to_string(),
        &at.height.to_string(),
        if popup { "1" } else { "0" },
    ];
    launched.spawn("python3", &args)
}

pub(super) fn paint_qt(launched: &mut Launched, title: &str, color: &str, at: &Geometry) -> u32 {
    let home = std::env::var("HOME").expect("HOME");
    let dir = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("a private session");
    let script = format!("{dir}/capture-qt-painter.py");
    std::fs::write(&script, QT_PAINTER).expect("the Qt painter writes");
    let python = format!("{home}/.local/ad-tools/qtvenv/bin/python");
    let libraries = format!("{home}/.local/ad-tools/root/usr/lib/x86_64-linux-gnu");
    let args = [
        script.as_str(),
        title,
        color,
        &at.x.to_string(),
        &at.y.to_string(),
        &at.width.to_string(),
        &at.height.to_string(),
    ];
    launched.spawn_with_env(
        &python,
        &args,
        &[
            ("QT_ACCESSIBILITY", "1".to_owned()),
            ("QT_LINUX_ACCESSIBILITY_ALWAYS_ON", "1".to_owned()),
            ("QT_QPA_PLATFORM", "xcb".to_owned()),
            ("LD_LIBRARY_PATH", libraries),
        ],
    )
}

pub(super) fn stop(pid: u32) {
    let status = Command::new("kill")
        .args(["-STOP", &pid.to_string()])
        .status()
        .expect("kill runs");
    assert!(status.success(), "pid {pid} could not be stopped");
}

pub(super) fn chrome(launched: &mut Launched) -> u32 {
    let dir = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("a private session");
    let profile = format!("{dir}/chrome-profile");
    let profile_flag = format!("--user-data-dir={profile}");
    launched.scratch.push(profile.clone().into());
    launched.spawn(
        "google-chrome",
        &[
            &profile_flag,
            "--no-first-run",
            "--no-default-browser-check",
            "--force-renderer-accessibility",
            "--ozone-platform=x11",
            "--disable-gpu",
            "--window-position=200,150",
            "--window-size=400,300",
            "--app=data:text/html,<title>adcap-chrome</title><body style='margin:0;background:%230000ff'></body>",
        ],
    )
}

pub(super) fn wait_for_x_window(title: &str) {
    let started = Instant::now();
    while started.elapsed() < SETTLE {
        let found = Command::new("xwininfo")
            .args(["-name", title])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
        if found {
            std::thread::sleep(Duration::from_millis(1_200));
            return;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("no X window titled {title} appeared");
}

pub(super) fn wait_for_listed(bus: &SharedBus, pid: u32) -> WindowInfo {
    let everything = WindowFilter {
        focused_only: false,
        app: None,
    };
    let started = Instant::now();
    while started.elapsed() < SETTLE {
        let listed = list_windows(bus, &everything, deadline(10_000)).unwrap_or_default();
        if listed.iter().any(|window| window.pid == pid) {
            std::thread::sleep(Duration::from_millis(1_200));
            let settled = list_windows(bus, &everything, deadline(10_000)).unwrap_or_default();
            if let Some(window) = settled.into_iter().find(|window| window.pid == pid) {
                return window;
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("pid {pid} never listed a window");
}

pub(super) struct Decoded {
    pub(super) width: usize,
    pub(super) height: usize,
    channels: usize,
    pixels: Vec<u8>,
}

impl Decoded {
    pub(super) fn at(&self, x: usize, y: usize) -> [u8; 3] {
        let start = (y * self.width + x) * self.channels;
        [
            self.pixels[start],
            self.pixels[start + 1],
            self.pixels[start + 2],
        ]
    }

    pub(super) fn count_of(&self, color: [u8; 3]) -> usize {
        (0..self.height)
            .flat_map(|y| (0..self.width).map(move |x| (x, y)))
            .filter(|&(x, y)| self.at(x, y) == color)
            .count()
    }

    pub(super) fn region_is(
        &self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        color: [u8; 3],
    ) -> bool {
        (y..y + height).all(|row| (x..x + width).all(|column| self.at(column, row) == color))
    }
}

pub(super) fn decode(image: &ImageBuffer) -> Decoded {
    let decoder = png::Decoder::new(Cursor::new(&image.data));
    let mut reader = decoder.read_info().expect("a PNG header");
    let mut pixels = vec![0; reader.output_buffer_size().expect("a buffer size")];
    let info = reader.next_frame(&mut pixels).expect("a PNG frame");
    let channels = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        other => panic!("unexpected PNG colour type {other:?}"),
    };
    assert_eq!(
        (info.width, info.height),
        (image.width, image.height),
        "the PNG header and the buffer agree"
    );
    Decoded {
        width: info.width as usize,
        height: info.height as usize,
        channels,
        pixels,
    }
}

pub(super) fn reason(error: &agent_desktop_core::AdapterError) -> String {
    error
        .details
        .as_ref()
        .and_then(|details| details["reason"].as_str())
        .unwrap_or("none")
        .to_owned()
}

pub(super) struct FixedLookup {
    pub(super) xid: u32,
}

impl WindowLookup for FixedLookup {
    fn resolve(&self, window: &WindowInfo, _: Deadline) -> Result<WindowInfo, AdapterError> {
        Ok(window.clone())
    }

    fn xid(&self, _: &WindowInfo, _: Deadline) -> XidOutcome {
        XidOutcome::One(self.xid)
    }

    fn displays(&self, deadline: Deadline) -> Result<Vec<DisplayInfo>, AdapterError> {
        crate::system::list_displays(deadline)
    }
}

pub(super) fn x_window_id(title: &str) -> u32 {
    let output = Command::new("xwininfo")
        .args(["-name", title])
        .output()
        .expect("xwininfo runs");
    let text = String::from_utf8_lossy(&output.stdout);
    let id = text
        .split("Window id: ")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .expect("xwininfo names the window");
    u32::from_str_radix(id.trim_start_matches("0x"), 16).expect("a window id")
}
