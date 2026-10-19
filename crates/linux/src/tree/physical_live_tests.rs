use std::time::Duration;

use agent_desktop_core::{
    Action, DragParams, ErrorCode, InputOps, KeyCombo, Modifier, NativeHandle, Point,
};

use super::live_launch::{Launched, nonce};
use super::live_offscreen_live_tests::scripts::{GTK3_LIST, GTK4_LIST, QT_LIST};
use super::physical_live_rig::{Kit, Scene, Tgt};
use super::physical_live_scripts::{
    GTK3_BUTTON, GTK3_DRAG, GTK3_GRAB, GTK4_BUTTON, QT_BUTTON, QT_ENTRY,
};
use crate::input::x11::live_tests::{GTK3, GTK4, device_state, window_center};

const ENTRY_ROLES: &[&str] = &["textfield", "text", "entry"];
const CLICK_ROLES: &[&str] = &["button"];

fn entry_scene(kit: Kit, source: &str) -> (Scene, Tgt) {
    let scene = Scene::open(kit, source, 2);
    let entry = scene.find(ENTRY_ROLES, None);
    (scene, entry)
}

fn combo(key: &str, modifiers: Vec<Modifier>) -> KeyCombo {
    KeyCombo {
        key: key.to_owned(),
        modifiers,
    }
}

fn focus_by_click(scene: &Scene, target: &Tgt) {
    scene
        .act(&target.handle, Action::Click)
        .expect("the focusing click is delivered");
    std::thread::sleep(Duration::from_millis(300));
}

fn type_text(kit: Kit, source: &str) {
    let (scene, entry) = entry_scene(kit, source);
    focus_by_click(&scene, &entry);
    let text = "h\u{e9}llo \u{65e5}\u{672c}";
    let result = scene
        .act(&entry.handle, Action::TypeText(text.to_owned()))
        .expect("the text is typed");
    assert!(result.steps.iter().all(|step| step.verified == Some(false)));
    let seen = scene.wait_log(|log| log == text);
    assert_eq!(seen, text, "the application did not receive the text");
    assert_eq!(scene.value_of(&entry).as_deref(), Some(text));
}

fn key_combo(kit: Kit, source: &str) {
    let (scene, entry) = entry_scene(kit, source);
    focus_by_click(&scene, &entry);
    scene
        .act(&entry.handle, Action::TypeText("abc".to_owned()))
        .expect("typed");
    scene.wait_log(|log| log == "abc");
    scene
        .act(
            &entry.handle,
            Action::PressKey(combo("a", vec![Modifier::Ctrl])),
        )
        .expect("select all");
    scene
        .act(&entry.handle, Action::TypeText("X".to_owned()))
        .expect("typed over the selection");
    assert_eq!(scene.wait_log(|log| log == "X"), "X");
    scene
        .act(&entry.handle, Action::PressKey(combo("BackSpace", vec![])))
        .expect("backspace");
    assert_eq!(scene.wait_log(str::is_empty), "");
}

fn click_button(kit: Kit, source: &str) {
    let scene = Scene::open(kit, source, 2);
    let button = scene.find(CLICK_ROLES, Some("Go"));
    scene
        .act(&button.handle, Action::Click)
        .expect("the click is delivered");
    let seen = scene.wait_log(|log| log == "clicked\n");
    assert_eq!(seen, "clicked\n", "the button did not report a click");
}

fn wheel(kit: Kit, source: &str, anchor: &str) {
    let scene = Scene::open(kit, source, 20);
    let row = scene.find(&["button", "option", "listitem"], Some(anchor));
    let before = scene.settled_y(&row);
    scene
        .act(
            &row.handle,
            Action::Scroll(agent_desktop_core::Direction::Down, 1),
        )
        .expect("the wheel is delivered");
    let moved = scene.settled_y(&row);
    assert!(before - moved > 30.0, "content moved {before} -> {moved}");
    scene
        .act(
            &row.handle,
            Action::Scroll(agent_desktop_core::Direction::Up, 1),
        )
        .expect("the wheel is delivered");
    let back = scene.settled_y(&row);
    assert!(
        (back - before).abs() < 3.0,
        "content returned to {back}, not {before}"
    );
}

fn pointer_buttons_down() -> bool {
    device_state("Virtual core XTEST pointer").contains("=down")
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_type_into_a_gtk3_entry_reads_back_with_remapped_characters() {
    type_text(Kit::Gtk, GTK3);
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_type_into_a_gtk4_entry_reads_back_with_remapped_characters() {
    type_text(Kit::Gtk, GTK4);
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_type_into_a_qt_entry_reads_back_with_remapped_characters() {
    type_text(Kit::Qt, QT_ENTRY);
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_key_combos_select_and_delete_in_a_gtk3_entry() {
    key_combo(Kit::Gtk, GTK3);
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_key_combos_select_and_delete_in_a_gtk4_entry() {
    key_combo(Kit::Gtk, GTK4);
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_key_combos_select_and_delete_in_a_qt_entry() {
    key_combo(Kit::Qt, QT_ENTRY);
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_a_click_at_the_bounds_centre_presses_a_gtk3_button() {
    click_button(Kit::Gtk, GTK3_BUTTON);
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_a_click_at_the_bounds_centre_presses_a_gtk4_button() {
    click_button(Kit::Gtk, GTK4_BUTTON);
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_a_click_at_the_bounds_centre_presses_a_qt_button() {
    click_button(Kit::Qt, QT_BUTTON);
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_the_wheel_scrolls_a_gtk3_list_and_back() {
    wheel(Kit::Gtk, GTK3_LIST, "Row 005");
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_the_wheel_scrolls_a_gtk4_list_and_back() {
    wheel(Kit::Gtk, GTK4_LIST, "Row 005");
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_the_wheel_scrolls_a_qt_list_and_back() {
    wheel(Kit::Qt, QT_LIST, "Item 005");
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_a_drag_presses_moves_and_releases_the_button() {
    let scene = Scene::open(Kit::Gtk, GTK3_DRAG, 1);
    let (cx, cy) = window_center("addrag");
    let (from, to) = (
        Point {
            x: f64::from(cx - 150),
            y: f64::from(cy),
        },
        Point {
            x: f64::from(cx + 150),
            y: f64::from(cy + 60),
        },
    );
    let lease = scene.lease();
    let params = DragParams {
        from: from.clone(),
        to: to.clone(),
        duration_ms: Some(300),
        drop_delay_ms: Some(100),
    };
    scene
        .adapter
        .drag(params, &lease)
        .expect("the drag is delivered");
    let seen = scene.wait_log(|log| log.contains("release"));
    let near = |line: &str, point: &Point| {
        let numbers: Vec<f64> = line
            .split_whitespace()
            .skip(1)
            .filter_map(|n| n.parse().ok())
            .collect();
        (numbers[0] - point.x).abs() < 3.0 && (numbers[1] - point.y).abs() < 3.0
    };
    let press = seen
        .lines()
        .find(|l| l.starts_with("press"))
        .expect("a press");
    let release = seen
        .lines()
        .find(|l| l.starts_with("release"))
        .expect("a release");
    assert!(near(press, &from), "{press} is not at {from:?}");
    assert!(near(release, &to), "{release} is not at {to:?}");
    assert!(seen.lines().filter(|l| *l == "move").count() >= 3, "{seen}");
    assert!(!pointer_buttons_down(), "a button is still down");
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_a_frame_that_is_not_active_refuses_with_window_active() {
    let (scene, entry) = entry_scene(Kit::Gtk, GTK3);
    let mut launched = Launched::new();
    let dir = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("session");
    let script = format!("{dir}/{}.py", nonce("decoy"));
    std::fs::write(&script, GTK3).expect("script");
    let decoy_log = format!("{script}.log");
    let pid = launched.spawn_with_env("python3", &[&script], &[("AD_LIVE_LOG", decoy_log.clone())]);
    let decoy = Scene::attach(launched, pid, decoy_log, 2);
    std::thread::sleep(Duration::from_secs(1));
    let error = scene
        .act(&entry.handle, Action::TypeText("zz".to_owned()))
        .expect_err("a frame that is not active refuses");
    assert_eq!(error.code, ErrorCode::ActionFailed, "{error:?}");
    let details = error.details.as_ref().expect("details");
    assert_eq!(details["check"], "window_active", "{details}");
    assert_eq!(details["physical_delivery_started"], false);
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(scene.log_text(), "");
    assert_eq!(decoy.log_text(), "");
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_keys_under_a_menu_grab_are_reported_unverified() {
    let mut launched = Launched::new();
    let dir = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("session");
    let script = format!("{dir}/{}.py", nonce("grab"));
    std::fs::write(&script, GTK3_GRAB).expect("script");
    let grab_log = format!("{script}.log");
    let pid = launched.spawn_with_env("python3", &[&script], &[("AD_LIVE_LOG", grab_log.clone())]);
    let grab = Scene::attach(launched, pid, grab_log, 1);
    std::thread::sleep(Duration::from_millis(1200));
    let (scene, entry) = entry_scene(Kit::Gtk, GTK3);
    std::thread::sleep(Duration::from_millis(800));
    let raised = std::process::Command::new("wmctrl")
        .args(["-a", "adx"])
        .status()
        .expect("wmctrl runs");
    assert!(raised.success());
    std::thread::sleep(Duration::from_millis(500));
    let net_active = std::process::Command::new("xprop")
        .args(["-root", "_NET_ACTIVE_WINDOW"])
        .output()
        .expect("xprop runs");
    assert!(
        !String::from_utf8_lossy(&net_active.stdout).contains("0x0"),
        "the window manager named no active window"
    );
    let refused = scene
        .act(&entry.handle, Action::TypeText("zq".to_owned()))
        .expect_err("the AT-SPI frame state is stricter than _NET_ACTIVE_WINDOW under a grab");
    assert_eq!(refused.details.expect("details")["check"], "window_active");
    assert_eq!(grab.log_text(), "");
    let combo = KeyCombo {
        key: "z".to_owned(),
        modifiers: vec![],
    };
    let result = scene
        .act(&NativeHandle::null(), Action::PressKey(combo))
        .expect("a global press has no gate");
    assert!(
        result.steps.iter().all(|step| step.verified == Some(false)),
        "{result:?}"
    );
    std::thread::sleep(Duration::from_millis(500));
    assert!(grab.log_text().contains("menukey"), "the menu got no key");
    assert_eq!(scene.log_text(), "");
}
