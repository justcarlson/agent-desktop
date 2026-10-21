use super::live_bench::Bench;
use super::live_scroll_tests::read;
use crate::tree::live_launch::{Launched, nonce};
use agent_desktop_core::{
    Action, ActionRequest, Deadline, DeliverySemantics, HitTestResult, ObservationOps, Point,
};

const APP: &str = r#"
import gi, json, pathlib, sys
gi.require_version('Gtk', '3.0')
from gi.repository import Gtk, Gdk
window = Gtk.Window(title=sys.argv[1])
window.set_default_size(300, 200)
window.move(100, 100)
late = sys.argv[3] == 'late'
counts = {'Target': 0, 'Cover': 0, 'double': 0, 'triple': 0, 'times': [], 'interval': Gtk.Settings.get_default().get_property('gtk-double-click-time')}
def record():
    path = pathlib.Path(sys.argv[2])
    temporary = path.with_suffix('.tmp')
    temporary.write_text(json.dumps(counts))
    temporary.replace(path)
def press(widget, event):
    if event.type == Gdk.EventType._2BUTTON_PRESS:
        counts['double'] += 1
    if event.type == Gdk.EventType._3BUTTON_PRESS:
        counts['triple'] += 1
    return False
def release(widget, event):
    counts['Target'] += 1
    counts['times'].append(event.time)
    if late and counts['Target'] == 1:
        cover.move(*window.get_position())
        cover.show_all()
        Gdk.Display.get_default().sync()
    record()
    return False
def intercepted(widget, event):
    counts['Cover'] += 1
    record()
    return False
target = Gtk.Button(label='Target')
target.connect('button-press-event', press)
target.connect('button-release-event', release)
window.add(target)
cover = Gtk.Window(type=Gtk.WindowType.POPUP, title='Cover')
cover.set_decorated(False)
cover.set_keep_above(True)
cover.set_default_size(300, 200)
interceptor = Gtk.Button(label='Cover')
interceptor.connect('button-release-event', intercepted)
cover.add(interceptor)
window.show_all()
record()
Gtk.main()
"#;

#[test]
#[ignore = "drives GTK in scripts/private-session.sh --wm"]
fn ref_multiclick_stops_at_a_changed_receiver_and_preserves_the_gesture() {
    for count in [2, 3] {
        for mode in ["stable", "late"] {
            let mut launched = Launched::new();
            let tag = nonce("click-receiver");
            let scratch = std::env::temp_dir().join(&tag);
            std::fs::create_dir_all(&scratch).unwrap();
            launched.scratch.push(scratch.clone());
            let state = scratch.join("state.json");
            let pid = launched.spawn("python3", &["-c", APP, &tag, state.to_str().unwrap(), mode]);
            let adapter = crate::LinuxAdapter::new();
            let frame = Bench::wait_for_frame(&adapter, &tag, pid, 20).expect("GTK window");
            let bench = Bench::attach(launched, frame);
            let target = bench.named("Target");
            assert!(
                bench
                    .wait_for(|| {
                        let deadline = Deadline::after(3_000).unwrap();
                        let bounds = bench
                            .adapter()
                            .get_element_bounds(&target, deadline)
                            .unwrap()?;
                        matches!(
                            bench
                                .adapter()
                                .hit_test(
                                    &target,
                                    Point {
                                        x: bounds.x + bounds.width / 2.0,
                                        y: bounds.y + bounds.height / 2.0
                                    },
                                    deadline
                                )
                                .unwrap(),
                            HitTestResult::ReachesTarget
                        )
                        .then_some(())
                    })
                    .is_some()
            );
            let action = if count == 2 {
                Action::DoubleClick
            } else {
                Action::TripleClick
            };
            let result = bench.act_with(&target, ActionRequest::headed(action));
            let sent = if mode == "late" { 1 } else { count };
            if mode == "late" {
                let error = result.expect_err("first click raises an interceptor");
                assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
                let details = error.details.expect("refusal evidence");
                assert_eq!(details["delivered_clicks"], 1);
                assert_eq!(details["physical_delivery_started"], true);
                assert_eq!(details["check"], "receives_events");
                assert_eq!(details["interceptor"]["name"], "Cover");
            } else {
                result.expect("stable target receives gesture");
            }
            assert!(
                bench
                    .wait_for(|| (read(&state)["Target"] == sent).then_some(()))
                    .is_some()
            );
            let observed = read(&state);
            assert_eq!(observed["Cover"], 0);
            if mode == "stable" {
                assert_eq!(observed["double"], 1, "the app recognized a double click");
                assert_eq!(observed["triple"], if count == 3 { 1 } else { 0 });
                let times = observed["times"].as_array().unwrap();
                for pair in times.windows(2) {
                    assert!(
                        pair[1].as_u64().unwrap() - pair[0].as_u64().unwrap()
                            < observed["interval"].as_u64().unwrap(),
                        "{observed}"
                    );
                }
            }
            eprintln!("{mode} {count}-click independent app receipt: {observed}");
        }
    }
}
