use std::time::Duration;

use agent_desktop_core::{Action, Direction, KeyCombo, Modifier};

use super::live_launch::{Launched, nonce};
use super::live_offscreen_live_tests::scripts::QT_LIST;
use super::physical_live_rig::{Kit, Scene, Tgt};
use crate::actions::scroll_into_view;

fn app_scene(program: &str, args: &[&str], min_nodes: usize) -> Scene {
    let mut launched = Launched::new();
    let pid = launched.spawn(program, args);
    Scene::attach(launched, pid, String::new(), min_nodes)
}

fn type_and_read(scene: &Scene, field: &Tgt, text: &str, replace: bool) -> String {
    scene
        .act(&field.handle, Action::Click)
        .expect("the focusing click is delivered");
    std::thread::sleep(Duration::from_millis(500));
    if replace {
        let select_all = KeyCombo {
            key: "a".to_owned(),
            modifiers: vec![Modifier::Ctrl],
        };
        scene
            .act(&field.handle, Action::PressKey(select_all))
            .expect("select all");
    }
    scene
        .act(&field.handle, Action::TypeText(text.to_owned()))
        .expect("the text is typed");
    for _ in 0..40 {
        if scene.value_of(field).is_some_and(|value| !value.is_empty()) {
            std::thread::sleep(Duration::from_millis(300));
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    scene.value_of(field).unwrap_or_default()
}

fn wheel_on(scene: &Scene, anchor: &Tgt, amount: u32) {
    let before = scene.settled_y(anchor);
    scene
        .act(&anchor.handle, Action::Scroll(Direction::Down, amount))
        .expect("the wheel is delivered");
    let moved = scene.settled_y(anchor);
    assert!(before - moved > 30.0, "content moved {before} -> {moved}");
    scene
        .act(&anchor.handle, Action::Scroll(Direction::Up, amount))
        .expect("the wheel is delivered");
    let back = scene.settled_y(anchor);
    assert!(
        (back - before).abs() < 3.0,
        "content returned to {back}, not {before}"
    );
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_text_editor_receives_typed_text_with_remapped_characters() {
    let scene = app_scene("gnome-text-editor", &[], 5);
    let field = scene.find(&["textfield"], None);
    let text = "h\u{e9}llo \u{65e5}\u{672c}";
    assert_eq!(type_and_read(&scene, &field, text, true), text);
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_calculator_evaluates_typed_keys() {
    let scene = app_scene("gnome-calculator", &[], 10);
    let display = scene.find(&["textfield"], None);
    let shown = type_and_read(&scene, &display, "6*7=", false);
    assert!(shown.contains("42"), "the display reads {shown:?}");
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_the_wheel_scrolls_the_files_list() {
    let dir = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("session");
    let files = format!("{dir}/{}", nonce("files"));
    std::fs::create_dir_all(&files).expect("dir");
    for i in 0..300 {
        std::fs::write(format!("{files}/file-{i:03}.txt"), "x").expect("file");
    }
    let scene = app_scene("nautilus", &["--new-window", &files], 20);
    let anchor = scene.find(&["cell"], Some("file-005.txt"));
    wheel_on(&scene, &anchor, 1);
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_qt_headless_reveal_has_no_container_while_the_headed_wheel_moves_the_content() {
    let scene = Scene::open(Kit::Qt, QT_LIST, 20);
    let far = scene.find(&["option"], Some("Item 040"));
    let near = scene.find(&["option"], Some("Item 005"));
    let error = {
        let lease = scene.lease();
        scroll_into_view(&scene.bus, &far.handle, &lease)
            .expect_err("a Qt list has no scroll container to drive headless")
    };
    let details = error.details.as_ref().expect("details");
    assert_eq!(details["cause"], "no_scroll_container", "{details}");
    let before = scene.adapter_bounds(&far);
    scene
        .act(&near.handle, Action::Scroll(Direction::Down, 13))
        .expect("the headed wheel is delivered");
    std::thread::sleep(Duration::from_millis(800));
    let after = scene.adapter_bounds(&far).expect("the item is on screen");
    assert!(
        before.is_none_or(|b| b.y - after.y > 300.0),
        "{before:?} -> {after:?}"
    );
}
