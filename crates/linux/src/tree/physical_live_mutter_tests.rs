use std::process::Command;
use std::time::Duration;

use agent_desktop_core::{Action, KeyCombo, NativeHandle};

use super::physical_live_rig::{Kit, Scene};
use super::physical_live_scripts::GTK3_NEVER_FOCUSED;
use super::physical_live_x11::{
    active_window, holds_for, wait_for_closed_overview, wait_for_viewable_client, wait_until,
};
use crate::actions::frame_active::frame_active;

const SETTLE: Duration = Duration::from_secs(15);

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

fn is_active(scene: &Scene) -> Option<bool> {
    let deadline = super::physical_live_rig::deadline(5_000);
    frame_active(&scene.bus, &scene.frame, deadline).expect("the frame state reads")
}

#[test]
#[ignore = "needs a private gnome-shell --x11 on the private display (see ledger L28-2 and L28-3)"]
fn live_a_raw_click_activates_a_window_only_once_the_overview_is_closed() {
    assert!(
        window_manager_name().contains("GNOME Shell"),
        "this case runs only under a private gnome-shell --x11"
    );
    let target = Scene::open(Kit::Gtk, GTK3_NEVER_FOCUSED, 2);
    wait_for_viewable_client(target.pid, SETTLE);
    let entry = target.find(&["textfield", "text", "entry"], None);
    assert_eq!(
        is_active(&target),
        Some(false),
        "the target never took focus"
    );
    let overview_window = active_window();
    target
        .act(&entry.handle, Action::Click)
        .expect("the raw click is delivered");
    holds_for(
        "the Overview took the click",
        Duration::from_secs(1),
        || is_active(&target) == Some(false) && active_window() == overview_window,
    );
    let refused = target.act(&entry.handle, Action::TypeText("no".to_owned()));
    assert!(refused.is_err(), "keys reached a window under the Overview");
    assert_eq!(target.log_text(), "");
    let escape = KeyCombo {
        key: "escape".to_owned(),
        modifiers: vec![],
    };
    target
        .act(&NativeHandle::null(), Action::PressKey(escape))
        .expect("the Overview closes");
    wait_for_closed_overview(SETTLE);
    wait_for_viewable_client(target.pid, SETTLE);
    target
        .act(&entry.handle, Action::Click)
        .expect("the raw click is delivered");
    wait_until("the click activating the frame", SETTLE, || {
        is_active(&target) == Some(true)
    });
    target
        .act(&entry.handle, Action::TypeText("ok".to_owned()))
        .expect("the keys follow the click");
    assert_eq!(target.wait_log(|log| log == "ok"), "ok");
}
