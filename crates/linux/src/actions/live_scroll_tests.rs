use std::path::Path;

use agent_desktop_core::{Action, DeliverySemantics};

use super::live_bench::Bench;
use crate::tree::live_launch::{Launched, nonce};

const APP: &str = r#"
import json, pathlib, sys
import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk, GLib
window = Gtk.Window(title=sys.argv[1])
window.connect("destroy", Gtk.main_quit)
pane = Gtk.ScrolledWindow()
pane.set_size_request(240, 200)
pane.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.ALWAYS)
column = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
for index in range(80):
    button = Gtk.Button(label="Far" if index == 79 else "Anchor" if index == 0 else "Row %s" % index)
    column.pack_start(button, False, False, 0)
    if index == 0:
        anchor = button
    if index == 79:
        far = button
pane.add(column)
window.add(pane)
adjustment = pane.get_vadjustment()
def record(*_):
    if sys.argv[3] == "withdraw" and adjustment.get_value() > 0:
        far.hide()
    path = pathlib.Path(sys.argv[2])
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps({"offset": adjustment.get_value(), "visible": far.get_visible(), "target_y": far.get_allocation().y, "target_height": far.get_allocated_height(), "viewport_height": pane.get_allocated_height()}))
    temporary.replace(path)
    return False
adjustment.connect("value-changed", record)
window.show_all()
anchor.grab_focus()
GLib.idle_add(record)
Gtk.main()
"#;

fn read(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("app state file")).unwrap()
}

fn open(mode: &str) -> (Bench, std::path::PathBuf) {
    let mut launched = Launched::new();
    let tag = nonce("scroll");
    let scratch = std::env::temp_dir().join(&tag);
    std::fs::create_dir_all(&scratch).unwrap();
    launched.scratch.push(scratch.clone());
    let state = scratch.join("state.json");
    let pid = launched.spawn("python3", &["-c", APP, &tag, state.to_str().unwrap(), mode]);
    let adapter = crate::LinuxAdapter::new();
    let frame = Bench::wait_for_frame(&adapter, &tag, pid, 20).expect("GTK window");
    let bench = Bench::attach(launched, frame);
    (bench, state)
}

#[test]
#[ignore = "drives GTK in scripts/private-session.sh --wm"]
fn a_scrolled_then_withdrawn_target_preserves_delivery_and_a_stable_target_arrives() {
    for mode in ["stable", "withdraw"] {
        let (bench, state) = open(mode);
        let far = bench.named("Far");
        assert_eq!(read(&state)["offset"], 0.0);
        let result = bench.act(&far, Action::ScrollTo);
        if mode == "withdraw" {
            let error = result.expect_err("app withdraws the target after scrolling");
            assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
            assert_eq!(read(&state)["visible"], false);
        } else {
            result.expect("target appears");
            let observed = read(&state);
            let offset = observed["offset"].as_f64().unwrap();
            let top = observed["target_y"].as_f64().unwrap() - offset;
            let bottom = top + observed["target_height"].as_f64().unwrap();
            assert!(
                top >= 0.0 && bottom <= observed["viewport_height"].as_f64().unwrap(),
                "app geometry: {observed}"
            );
            let satisfied = bench
                .act(&far, Action::ScrollTo)
                .expect("already on screen");
            assert_eq!(satisfied.disposition(), DeliverySemantics::not_delivered());
            assert_eq!(read(&state)["offset"], offset);
        }
        assert!(read(&state)["offset"].as_f64().unwrap() > 0.0);
    }
}

struct Paused(u32);

impl Drop for Paused {
    fn drop(&mut self) {
        let _ = std::process::Command::new("kill")
            .args(["-CONT", &self.0.to_string()])
            .status();
    }
}

#[test]
#[ignore = "drives GTK in scripts/private-session.sh --wm"]
fn scroll_to_timeout_after_dispatch_is_uncertain_and_a_false_reply_is_absent() {
    use super::scroll_into_view::ScrollPort;
    use agent_desktop_core::{Deadline, ErrorCode};
    let (bench, state) = open("stable");
    let far = bench.named("Far");
    let element = crate::tree::atspi_element(&far).unwrap();
    let bus = bench.adapter().bus();
    let before = read(&state);
    let mut port =
        super::scroll_bus::BusPort::new(bus, &far, element, Deadline::after(3_000).unwrap());
    assert!(!port.scroll_to().expect("GTK explicitly answers false"));
    assert_eq!(read(&state), before);
    let pid = bench.frame().pid();
    assert!(
        std::process::Command::new("kill")
            .args(["-STOP", &pid.to_string()])
            .status()
            .unwrap()
            .success()
    );
    let paused = Paused(pid);
    assert!(
        bench
            .wait_for(|| crate::system::proc_stat::is_stopped(pid).then_some(()))
            .is_some()
    );
    let mut port =
        super::scroll_bus::BusPort::new(bus, &far, element, Deadline::after(250).unwrap());
    let error = port.scroll_to().expect_err("no answer after dispatch");
    assert_eq!(error.code, ErrorCode::Timeout);
    assert_eq!(error.disposition, DeliverySemantics::uncertain());
    drop(paused);
    assert!(
        bench
            .wait_for(|| (!crate::system::proc_stat::is_stopped(pid)).then_some(()))
            .is_some()
    );
    assert_eq!(read(&state), before);
}

const WHEEL_APP: &str = r#"
import json, pathlib, sys
from PyQt6.QtCore import QPoint, QTimer, Qt
from PyQt6.QtWidgets import QApplication, QWidget, QListWidget, QPushButton
app = QApplication(sys.argv)
window = QWidget()
window.setWindowTitle(sys.argv[1])
window.setGeometry(100, 100, 300, 200)
counts = {"Target": 0, "Cover": 0, "covered": True}
def record():
    counts["offset"] = target.verticalScrollBar().value()
    path = pathlib.Path(sys.argv[2])
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(counts))
    temporary.replace(path)
class Target(QListWidget):
    def wheelEvent(self, event):
        counts["Target"] += 1
        super().wheelEvent(event)
        record()
class Cover(QPushButton):
    def wheelEvent(self, event):
        counts["Cover"] += 1
        event.accept()
        record()
target = Target(window)
target.setAccessibleName("Target")
target.setGeometry(0, 0, 300, 200)
target.addItems(["Row %s" % i for i in range(100)])
cover = Cover("Cover", window)
cover.setWindowFlags(Qt.WindowType.Window | Qt.WindowType.FramelessWindowHint | Qt.WindowType.WindowStaysOnTopHint)
cover.setWindowTitle("Cover")
cover.setAccessibleName("Cover")
cover.setGeometry(100, 100, 300, 200)
def poll():
    cover.move(target.mapToGlobal(QPoint(0, 0)))
    if pathlib.Path(sys.argv[3]).exists():
        cover.hide()
        counts["covered"] = False
        record()
        timer.stop()
timer = QTimer()
timer.timeout.connect(poll)
timer.start(50)
window.show()
cover.show()
record()
sys.exit(app.exec())
"#;

#[test]
#[ignore = "drives Qt in scripts/private-session.sh --wm"]
fn ref_wheel_refuses_a_covering_window_and_reaches_the_uncovered_target() {
    use agent_desktop_core::{
        ActionRequest, Deadline, Direction, ErrorCode, HitTestResult, ObservationOps, Point,
    };
    let mut launched = Launched::new();
    let tag = nonce("wheel-cover");
    let scratch = std::env::temp_dir().join(&tag);
    std::fs::create_dir_all(&scratch).unwrap();
    launched.scratch.push(scratch.clone());
    let state = scratch.join("state.json");
    let uncover = scratch.join("uncover");
    let (python, environment) = super::live_bench_env::qt_environment().expect("Qt fixture");
    let pid = launched.spawn_with_env(
        &python,
        &[
            "-c",
            WHEEL_APP,
            &tag,
            state.to_str().unwrap(),
            uncover.to_str().unwrap(),
        ],
        &environment,
    );
    let adapter = crate::LinuxAdapter::new();
    let frame = Bench::wait_for_frame(&adapter, &tag, pid, 20).expect("Qt window");
    let bench = Bench::attach(launched, frame);
    let target = bench.named("Target");
    let hit = || {
        let deadline = Deadline::after(3_000).unwrap();
        let bounds = bench
            .adapter()
            .get_element_bounds(&target, deadline)
            .unwrap()
            .unwrap();
        bench
            .adapter()
            .hit_test(
                &target,
                Point {
                    x: bounds.x + bounds.width / 2.0,
                    y: bounds.y + bounds.height / 2.0,
                },
                deadline,
            )
            .unwrap()
    };
    let intercepted = || matches!(hit(), HitTestResult::InterceptedBy { name, .. } if name.as_deref() == Some("Cover"));
    assert!(
        bench.wait_for(|| intercepted().then_some(())).is_some(),
        "the native cover is mapped above the target: {:?}",
        hit()
    );
    let wheel = || ActionRequest::headed(Action::Scroll(Direction::Down, 1));
    let error = bench
        .act_with(&target, wheel())
        .expect_err("covering window intercepts wheel");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    let details = error.details.expect("interception details");
    assert_eq!(details["check"], "receives_events");
    assert_eq!(details["interceptor"]["name"], "Cover");
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert_eq!(read(&state)["Target"], 0);
    assert_eq!(read(&state)["Cover"], 0);
    assert_eq!(read(&state)["offset"], 0);
    std::fs::write(&uncover, "").unwrap();
    assert!(
        bench
            .wait_for(|| (read(&state)["covered"] == false).then_some(()))
            .is_some()
    );
    assert!(
        bench
            .wait_for(|| matches!(hit(), HitTestResult::ReachesTarget).then_some(()))
            .is_some()
    );
    bench.act_with(&target, wheel()).expect("uncovered target");
    assert!(
        bench
            .wait_for(|| (read(&state)["Target"] == 1).then_some(()))
            .is_some()
    );
    assert_eq!(read(&state)["Cover"], 0);
    assert!(read(&state)["offset"].as_i64().unwrap() > 0);
}
