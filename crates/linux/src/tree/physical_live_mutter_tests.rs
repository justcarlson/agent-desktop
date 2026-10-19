use std::process::Command;
use std::time::{Duration, Instant};

use agent_desktop_core::{Action, KeyCombo, NativeHandle};

use super::physical_live_rig::{Kit, Scene};
use super::physical_live_scripts::GTK3_NEVER_FOCUSED;
use super::physical_live_shell::X11Shell;
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

const CHILD_ENV: &str = "AD_MUTTER_CHILD";
const CHILD_BUDGET: Duration = Duration::from_secs(120);
const THIS_TEST: &str = "tree::physical_live_mutter_tests::live_a_raw_click_activates_a_window_only_once_the_overview_is_closed";

#[test]
#[ignore = "drives its own private gnome-shell --x11; run through scripts/private-session.sh --wm"]
fn live_a_raw_click_activates_a_window_only_once_the_overview_is_closed() {
    if std::env::var(CHILD_ENV).is_ok() {
        click_scenario();
        return;
    }
    let shell = X11Shell::start();
    let mut child = Command::new(std::env::current_exe().expect("exe"))
        .args(["--exact", THIS_TEST, "--include-ignored", "--nocapture"])
        .env("DISPLAY", &shell.display)
        .env("AD_OWN_DISPLAY", &shell.display)
        .env(CHILD_ENV, "1")
        .spawn()
        .expect("the scenario process starts");
    let until = Instant::now() + CHILD_BUDGET;
    let status = loop {
        if let Some(status) = child.try_wait().expect("the scenario process is polled") {
            break Some(status);
        }
        if Instant::now() >= until {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(200));
    };
    drop(shell);
    assert!(
        status.is_some_and(|status| status.success()),
        "the scenario failed or did not finish in {CHILD_BUDGET:?}: {status:?}"
    );
}

fn click_scenario() {
    assert!(
        window_manager_name().contains("GNOME Shell"),
        "the scenario runs only under its own gnome-shell --x11"
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
        .act_physical(&entry.handle, Action::Click)
        .expect("the raw click is delivered");
    holds_for(
        "the Overview took the click",
        Duration::from_secs(1),
        || is_active(&target) == Some(false) && active_window() == overview_window,
    );
    let refused = target.act_physical(&entry.handle, Action::TypeText("no".to_owned()));
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
        .act_physical(&entry.handle, Action::Click)
        .expect("the raw click is delivered");
    wait_until("the click activating the frame", SETTLE, || {
        is_active(&target) == Some(true)
    });
    target
        .act_physical(&entry.handle, Action::TypeText("ok".to_owned()))
        .expect("the keys follow the click");
    assert_eq!(target.wait_log(|log| log == "ok"), "ok");
}
