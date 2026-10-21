use super::live_bench::Bench;
use super::live_scroll_tests::read;
use crate::tree::live_launch::{Launched, nonce};
use agent_desktop_core::{Action, DeliverySemantics};

const WHEEL_APP: &str = r#"
import json, pathlib, sys
from PyQt6.QtCore import QPoint, QTimer, Qt
from PyQt6.QtWidgets import QApplication, QWidget, QListWidget, QPushButton
app = QApplication(sys.argv)
window = QWidget()
window.setWindowTitle(sys.argv[1])
window.setGeometry(100, 100, 300, 200)
late = sys.argv[4] == "late"
counts = {"Target": 0, "Cover": 0, "covered": not late}
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
        if late and counts["Target"] == 10:
            cover.move(target.mapToGlobal(QPoint(0, 0)))
            cover.show()
            app.processEvents()
            counts["covered"] = True
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
app.processEvents()
if sys.argv[5] == "up":
    target.verticalScrollBar().setValue(target.verticalScrollBar().maximum())
counts["initial_offset"] = target.verticalScrollBar().value()
if not late:
    cover.show()
record()
sys.exit(app.exec())
"#;

#[test]
#[ignore = "drives Qt in scripts/private-session.sh --wm"]
fn ref_wheel_refuses_a_covering_window_and_reaches_the_uncovered_target() {
    for mode in ["covered", "late"] {
        for direction in [
            agent_desktop_core::Direction::Down,
            agent_desktop_core::Direction::Up,
        ] {
            use agent_desktop_core::{
                ActionRequest, Deadline, ErrorCode, HitTestResult, ObservationOps, Point,
            };
            let mut launched = Launched::new();
            let tag = nonce("wheel-cover");
            let scratch = std::env::temp_dir().join(&tag);
            std::fs::create_dir_all(&scratch).unwrap();
            launched.scratch.push(scratch.clone());
            let state = scratch.join("state.json");
            let uncover = scratch.join("uncover");
            let (python, environment) = super::live_bench_env::qt_environment();
            let pid = launched.spawn_with_env(
                &python,
                &[
                    "-c",
                    WHEEL_APP,
                    &tag,
                    state.to_str().unwrap(),
                    uncover.to_str().unwrap(),
                    mode,
                    if matches!(direction, agent_desktop_core::Direction::Up) {
                        "up"
                    } else {
                        "down"
                    },
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
                bench
                    .wait_for(|| (if mode == "late" {
                        matches!(hit(), HitTestResult::ReachesTarget)
                    } else {
                        intercepted()
                    })
                    .then_some(()))
                    .is_some(),
                "the native cover is mapped above the target: {:?}",
                hit()
            );
            let wheel = || ActionRequest::headed(Action::Scroll(direction.clone(), 21));
            let error = bench
                .act_with(&target, wheel())
                .expect_err("covering window intercepts wheel");
            assert_eq!(error.code, ErrorCode::ActionFailed);
            let details = error.details.expect("interception details");
            assert_eq!(details["check"], "receives_events");
            assert_eq!(details["interceptor"]["name"], "Cover");
            let sent = if mode == "late" { 10 } else { 0 };
            assert_eq!(details["delivered_clicks"], sent);
            assert_eq!(
                error.disposition,
                if mode == "late" {
                    DeliverySemantics::delivered_unverified()
                } else {
                    DeliverySemantics::not_delivered()
                }
            );
            assert_eq!(read(&state)["Target"], sent);
            assert_eq!(read(&state)["Cover"], 0);
            if mode == "covered" {
                assert_eq!(read(&state)["offset"], read(&state)["initial_offset"]);
            }
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
                    .wait_for(|| (read(&state)["Target"] == sent + 21).then_some(()))
                    .is_some()
            );
            assert_eq!(read(&state)["Cover"], 0);
            let observed = read(&state);
            let offset = observed["offset"].as_i64().unwrap();
            let initial = observed["initial_offset"].as_i64().unwrap();
            assert!(if matches!(direction, agent_desktop_core::Direction::Up) {
                offset < initial
            } else {
                offset > initial
            });
        }
    }
}
