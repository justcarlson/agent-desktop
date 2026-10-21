use std::process::Command;
use std::time::Duration;

use serde_json::Value;

use crate::tree::live_launch::Launched;
use crate::tree::live_rig::wait_until;

fn cli(binary: &std::ffi::OsStr, args: &[&str]) -> Value {
    let output = Command::new(binary)
        .args(args)
        .output()
        .expect("candidate CLI");
    serde_json::from_slice(&output.stdout).expect("JSON envelope")
}

fn enabled_button(node: &Value, name: &str) -> Option<String> {
    if node["role"] == "button"
        && node["name"] == name
        && !node["states"]
            .as_array()
            .is_some_and(|states| states.iter().any(|state| state == "disabled"))
    {
        return node["ref_id"].as_str().map(str::to_owned);
    }
    node["children"]
        .as_array()?
        .iter()
        .find_map(|child| enabled_button(child, name))
}

fn xprop(args: &[&str]) -> String {
    let output = Command::new("xprop")
        .args(args)
        .output()
        .expect("native window oracle");
    assert!(output.status.success());
    String::from_utf8(output.stdout).expect("X properties")
}

fn window_of(pid: u32) -> Option<String> {
    xprop(&["-root", "_NET_CLIENT_LIST"])
        .split('#')
        .nth(1)?
        .split(',')
        .map(str::trim)
        .find(|id| {
            xprop(&["-id", id, "_NET_WM_PID"])
                .split_whitespace()
                .last()
                .and_then(|text| text.parse::<u32>().ok())
                == Some(pid)
        })
        .map(str::to_owned)
}

#[test]
#[ignore = "private X11 desktop and AGENT_DESKTOP_TEST_BINARY pointing to the candidate CLI"]
fn live_a_fresh_files_click_changes_native_window_state() {
    let binary = std::env::var_os("AGENT_DESKTOP_TEST_BINARY").expect("candidate CLI path");
    let private = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("private session");
    let mut launched = Launched::new();
    let pid = launched.spawn("nautilus", &["--new-window", &private]);
    let mut target = None;
    assert!(
        wait_until(Duration::from_secs(20), || {
            let snapshot = cli(&binary, &["snapshot", "--app", "nautilus", "-i"]);
            target = enabled_button(&snapshot["data"]["tree"], "Maximize");
            target.is_some()
        }),
        "Files publishes an enabled Maximize button"
    );
    let window = window_of(pid).expect("native window of the launched process");
    assert!(!xprop(&["-id", &window, "_NET_WM_STATE"]).contains("MAXIMIZED"));
    let answer = cli(&binary, &["click", &target.expect("button ref")]);
    assert_eq!(answer["ok"], true, "{answer}");
    assert!(
        wait_until(Duration::from_secs(3), || {
            let state = xprop(&["-id", &window, "_NET_WM_STATE"]);
            state.contains("_NET_WM_STATE_MAXIMIZED_VERT")
                && state.contains("_NET_WM_STATE_MAXIMIZED_HORZ")
        }),
        "the window manager must observe the click's effect"
    );
}
