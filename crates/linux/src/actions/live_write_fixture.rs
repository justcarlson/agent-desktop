use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use agent_desktop_core::Deadline;

use crate::tree::live_launch::{Launched, nonce};

const WIDGETS_GTK3: &str = r#"
window = Gtk.Window(title="adw-" + sys.argv[1])
outer = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
flag = Gtk.CheckButton(label="Flag")
victim = Gtk.Button(label="Victim")
entry = Gtk.Entry()
entry.set_text("start")
scale = Gtk.Scale.new_with_range(Gtk.Orientation.HORIZONTAL, 0, 100, 1)
scale.set_value(10)
progress = Gtk.ProgressBar()
progress.set_fraction(0.5)
for part in (flag, victim, entry, scale, progress):
    outer.pack_start(part, False, False, 0)
window.add(outer)
parts.update(window=window, victim=victim)
window.connect("destroy", Gtk.main_quit)
GLib.timeout_add(100, poll)
window.show_all()
Gtk.main()
"#;

const WIDGETS_GTK4: &str = r#"
def activate(app):
    window = Gtk.ApplicationWindow(application=app, title="adw-" + sys.argv[1])
    outer = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
    flag = Gtk.CheckButton(label="Flag")
    victim = Gtk.Button(label="Victim")
    entry = Gtk.Entry()
    entry.set_text("start")
    scale = Gtk.Scale.new_with_range(Gtk.Orientation.HORIZONTAL, 0, 100, 1)
    scale.set_value(10)
    progress = Gtk.ProgressBar()
    progress.set_fraction(0.5)
    for part in (flag, victim, entry, scale, progress):
        outer.append(part)
    window.set_child(outer)
    parts.update(window=window, outer=outer, victim=victim)
    GLib.timeout_add(100, poll)
    window.present()

app = Gtk.Application(application_id="org.agentdesktop.WriteFixture4")
app.connect("activate", activate)
app.run([])
"#;

const HEAD_GTK3: &str = r#"
import sys
import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk, GLib

commands = sys.argv[2]
parts = {}

def run(words):
    if words[0] == "remove":
        parts["victim"].destroy()

def poll():
    try:
        text = open(commands).read().strip()
    except OSError:
        return True
    if text:
        open(commands, "w").close()
        for line in text.splitlines():
            run(line.split())
    return True
"#;

const HEAD_GTK4: &str = r#"
import sys
import gi
gi.require_version("Gtk", "4.0")
from gi.repository import Gtk, GLib

commands = sys.argv[2]
parts = {}

def run(words):
    if words[0] == "remove":
        parts["outer"].remove(parts["victim"])

def poll():
    try:
        text = open(commands).read().strip()
    except OSError:
        return True
    if text:
        open(commands, "w").close()
        for line in text.splitlines():
            run(line.split())
    return True
"#;

const QT: &str = r#"
import sys
from PyQt6.QtCore import QTimer, Qt
from PyQt6.QtWidgets import (QApplication, QMainWindow, QWidget, QVBoxLayout, QPushButton,
                             QLineEdit, QCheckBox, QSlider, QProgressBar)
commands = sys.argv[2]
app = QApplication(sys.argv)
window = QMainWindow()
window.setWindowTitle("adw-" + sys.argv[1])
central = QWidget()
column = QVBoxLayout(central)
flag = QCheckBox("Flag")
victim = QPushButton("Victim")
entry = QLineEdit()
entry.setText("start")
scale = QSlider(Qt.Orientation.Horizontal)
scale.setRange(0, 100)
scale.setValue(10)
progress = QProgressBar()
progress.setRange(0, 100)
progress.setValue(50)
for part in (flag, victim, entry, scale, progress):
    column.addWidget(part)
window.setCentralWidget(central)
window.setGeometry(100, 100, 480, 360)

def poll():
    try:
        text = open(commands).read().strip()
    except OSError:
        return
    if text:
        open(commands, "w").close()
        for line in text.splitlines():
            if line.split()[0] == "remove":
                victim.deleteLater()

timer = QTimer()
timer.timeout.connect(poll)
timer.start(100)
window.show()
sys.exit(app.exec())
"#;

pub(super) fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).expect("the deadline builds")
}

pub(super) fn wait_until(limit: Duration, mut check: impl FnMut() -> bool) -> bool {
    let started = Instant::now();
    while started.elapsed() < limit {
        if check() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    false
}

pub(super) fn signal(pid: u32, name: &str) {
    let _ = Command::new("kill")
        .args([&format!("-{name}"), &pid.to_string()])
        .status();
}

pub(super) struct Suspended(u32);

impl Suspended {
    pub(super) fn new(pid: u32) -> Self {
        signal(pid, "STOP");
        Self(pid)
    }
}

impl Drop for Suspended {
    fn drop(&mut self) {
        signal(self.0, "CONT");
    }
}

pub(super) struct App {
    pub(super) pid: u32,
    commands: PathBuf,
    launched: Launched,
}

impl App {
    pub(super) fn send(&self, line: &str) {
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&self.commands)
            .expect("the command file opens");
        writeln!(file, "{line}").expect("the command is written");
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.launched.stop();
    }
}

fn qt_environment() -> (String, Vec<(&'static str, String)>) {
    let home = std::env::var("HOME").expect("HOME is set");
    let python = format!("{home}/.local/ad-tools/qtvenv/bin/python");
    assert!(
        std::path::Path::new(&python).exists(),
        "the Qt test environment was not found at {python}"
    );
    let libraries = format!("{home}/.local/ad-tools/root/usr/lib/x86_64-linux-gnu");
    let library_path = match std::env::var("LD_LIBRARY_PATH") {
        Ok(existing) if !existing.is_empty() => format!("{libraries}:{existing}"),
        _ => libraries,
    };
    (
        python,
        vec![
            ("QT_ACCESSIBILITY", "1".to_owned()),
            ("QT_LINUX_ACCESSIBILITY_ALWAYS_ON", "1".to_owned()),
            ("QT_QPA_PLATFORM", "xcb".to_owned()),
            ("LD_LIBRARY_PATH", library_path),
        ],
    )
}

pub(super) fn launch(toolkit: &str) -> App {
    let mut launched = Launched::new();
    let dir = std::env::temp_dir().join(nonce("write"));
    std::fs::create_dir_all(&dir).expect("the fixture directory is made");
    launched.scratch.push(dir.clone());
    let source = match toolkit {
        "gtk3" => format!("{HEAD_GTK3}{WIDGETS_GTK3}"),
        "gtk4" => format!("{HEAD_GTK4}{WIDGETS_GTK4}"),
        _ => QT.to_owned(),
    };
    let script = dir.join("app.py");
    std::fs::write(&script, source).expect("the script is written");
    let commands = dir.join("commands");
    std::fs::write(&commands, "").expect("the command file is written");
    let (program, env) = if toolkit == "qt" {
        qt_environment()
    } else {
        ("python3".to_owned(), Vec::new())
    };
    let script_arg = script.to_string_lossy().into_owned();
    let commands_arg = commands.to_string_lossy().into_owned();
    let pid = launched.spawn_with_env(&program, &[&script_arg, "live", &commands_arg], &env);
    App {
        pid,
        commands,
        launched,
    }
}
