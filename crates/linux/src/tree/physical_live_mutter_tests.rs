use std::process::Command;
use std::time::Duration;

use agent_desktop_core::{Action, KeyCombo, NativeHandle};

use super::physical_live_rig::{Kit, Scene};
use super::physical_live_scripts::GTK3_NEVER_FOCUSED;
use crate::actions::frame_active::frame_active;
use crate::input::x11::live_tests::GTK3;

fn xprop(args: &[&str]) -> String {
    let output = Command::new("xprop")
        .args(args)
        .output()
        .expect("xprop runs");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn window_manager_name() -> String {
    let check = xprop(&["-root", "_NET_SUPPORTING_WM_CHECK"]);
    let id = check
        .split_whitespace()
        .last()
        .unwrap_or_default()
        .to_owned();
    xprop(&["-id", &id, "_NET_WM_NAME"])
}

fn active_window() -> String {
    xprop(&["-root", "_NET_ACTIVE_WINDOW"])
        .split_whitespace()
        .last()
        .unwrap_or_default()
        .to_owned()
}

fn is_active(scene: &Scene) -> Option<bool> {
    let deadline = super::physical_live_rig::deadline(5_000);
    frame_active(&scene.bus, &scene.frame, deadline).expect("the frame state reads")
}

#[test]
#[ignore = "needs a private gnome-shell --x11 on the private display (see ledger L28-2 and L28-3)"]
fn live_a_raw_click_gives_a_never_focused_window_its_first_focus_under_mutter() {
    assert!(
        window_manager_name().contains("GNOME Shell"),
        "this case runs only under a private gnome-shell --x11"
    );
    let decoy = Scene::open(Kit::Gtk, GTK3, 2);
    let escape = KeyCombo {
        key: "escape".to_owned(),
        modifiers: vec![],
    };
    decoy
        .act(&NativeHandle::null(), Action::PressKey(escape))
        .expect("the Overview closes");
    std::thread::sleep(Duration::from_secs(1));
    assert_eq!(is_active(&decoy), Some(true), "the decoy holds focus");
    let decoy_window = active_window();
    let target = Scene::open(Kit::Gtk, GTK3_NEVER_FOCUSED, 2);
    std::thread::sleep(Duration::from_secs(1));
    assert_eq!(
        is_active(&target),
        Some(false),
        "the target never took focus"
    );
    assert_eq!(active_window(), decoy_window);
    let entry = target.find(&["textfield", "text", "entry"], None);
    target
        .act(&entry.handle, Action::Click)
        .expect("the raw click is delivered");
    std::thread::sleep(Duration::from_secs(1));
    assert_eq!(is_active(&target), Some(true), "the click gave it focus");
    assert_ne!(active_window(), decoy_window);
    target
        .act(&entry.handle, Action::TypeText("ok".to_owned()))
        .expect("the keys follow the click");
    assert_eq!(target.wait_log(|log| log == "ok"), "ok");
    assert_eq!(decoy.log_text(), "");
}
