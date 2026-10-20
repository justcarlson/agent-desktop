use std::process::Command;
use std::time::{Duration, Instant};

use agent_desktop_core::{ErrorCode, WindowInfo};

use super::lift_live_tests::*;
use super::*;
use crate::tree::live_launch::Launched;

fn wait_active(pid: u32) -> bool {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(8) {
        if active_pid() == Some(pid) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

fn user_time(xid: &str) -> String {
    xprop(&["-id", xid, "_NET_WM_USER_TIME"])
        .split('=')
        .nth(1)
        .unwrap_or("unset")
        .trim()
        .to_owned()
}

fn report(label: &str, run: usize, bus: &SharedBus, target: &WindowInfo) {
    let xid = xid_of(target.pid.get());
    let before = xprop(&["-root", "_NET_ACTIVE_WINDOW"]);
    let user = user_time(&xid);
    let result = focus(bus, target);
    let detail = match &result {
        Ok(()) => "granted".to_owned(),
        Err(error) => format!(
            "{} cause={}",
            error.message,
            error
                .details
                .as_ref()
                .map_or("none", |d| d["cause"].as_str().unwrap_or("none"))
        ),
    };
    let state = xprop(&["-id", &xid, "_NET_WM_STATE"]);
    let line = format!(
        "F1 {label} run{run}: wm_state=[{}] before=[{}] target_user_time={user} -> {detail}; after=[{}] target_active={}",
        state.trim(),
        before.trim(),
        xprop(&["-root", "_NET_ACTIVE_WINDOW"]).trim(),
        active_pid() == Some(target.pid.get())
    );
    println!("{line}");
    let honest_refusal = window_manager().contains("GNOME Shell")
        && result.as_ref().is_err_and(|error| {
            error.details.as_ref().is_some_and(|details| {
                matches!(
                    details["cause"].as_str(),
                    Some("activation_refused" | "activation_not_granted")
                )
            })
        });
    assert!(result.is_ok() || honest_refusal, "{line}");
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
fn focus_window_live_f1_never_focused_conditions_are_measured() {
    close_overview();
    println!("F1 manager: {}", window_manager());
    for run in 1..=3 {
        {
            close_overview();
            let bus = SharedBus::default();
            let mut launched = Launched::new();
            let other = spawn(&mut launched, &bus, "other");
            if !wait_active(other.pid.get()) {
                println!("F1 precondition: a fresh window was not made active by the manager");
            }
            let target = spawn_with(&mut launched, &bus, "target", true);
            std::thread::sleep(Duration::from_secs(2));
            report("a never-focused, other active", run, &bus, &target);
        }
        {
            close_overview();
            let bus = SharedBus::default();
            let mut launched = Launched::new();
            let target = spawn(&mut launched, &bus, "target");
            if !wait_active(target.pid.get()) {
                println!("F1 precondition: a fresh window was not made active by the manager");
            }
            let other = spawn(&mut launched, &bus, "other");
            if !wait_active(other.pid.get()) {
                println!("F1 precondition: a fresh window was not made active by the manager");
            }
            std::thread::sleep(Duration::from_secs(1));
            report("b held-focus-before, other active", run, &bus, &target);
        }
        {
            close_overview();
            let bus = SharedBus::default();
            let mut launched = Launched::new();
            let target = spawn(&mut launched, &bus, "target");
            if !wait_active(target.pid.get()) {
                println!("F1 precondition: a fresh window was not made active by the manager");
            }
            let other = spawn(&mut launched, &bus, "other");
            if !wait_active(other.pid.get()) {
                println!("F1 precondition: a fresh window was not made active by the manager");
            }
            std::thread::sleep(Duration::from_secs(1));
            let _ = tool("xdotool").args(["key", "shift"]).status();
            report(
                "d held-focus-before, other active, input just before",
                run,
                &bus,
                &target,
            );
        }
        {
            close_overview();
            let bus = SharedBus::default();
            let mut launched = Launched::new();
            let target = spawn_with(&mut launched, &bus, "target", true);
            std::thread::sleep(Duration::from_secs(2));
            report("c never-focused, nothing active", run, &bus, &target);
        }
    }
}

fn names_of_active() -> String {
    let active = xprop(&["-root", "_NET_ACTIVE_WINDOW"]);
    let id = active
        .split_whitespace()
        .last()
        .unwrap_or_default()
        .to_owned();
    xprop(&["-id", &id, "WM_NAME"])
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
fn focus_window_live_a_modal_dialog_is_named_and_the_window_is_not_reported_active() {
    close_overview();
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let owner = spawn_env(&mut launched, &bus, "owner", false, true);
    std::thread::sleep(Duration::from_secs(2));
    let other = spawn(&mut launched, &bus, "other");
    let _ = focus(&bus, &other);
    let error = focus(&bus, &owner).expect_err("a modal dialog redirects the focus");
    let details = error.details.as_ref().expect("details");
    println!(
        "focus_window_live modal: {details} active={}",
        names_of_active()
    );
    assert_eq!(details["cause"], "blocked_by_modal_dialog");
    assert_eq!(details["dialog"], "adl-dialog");
    assert!(names_of_active().contains("adl-dialog"));
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
fn focus_window_live_a_stopped_application_is_refused_before_any_request() {
    close_overview();
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let target = spawn(&mut launched, &bus, "target");
    let _other = spawn(&mut launched, &bus, "other");
    let pid = target.pid.get().to_string();
    assert!(
        Command::new("kill")
            .args(["-STOP", &pid])
            .status()
            .expect("kill runs")
            .success()
    );
    let before = xprop(&["-root", "_NET_ACTIVE_WINDOW"]);
    let started = Instant::now();
    let result = focus(&bus, &target);
    let _ = Command::new("kill").args(["-CONT", &pid]).status();
    let error = result.expect_err("a stopped application is refused");
    assert_eq!(error.code, ErrorCode::AppUnresponsive);
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(xprop(&["-root", "_NET_ACTIVE_WINDOW"]), before);
}

fn current_desktop() -> String {
    xprop(&["-root", "_NET_CURRENT_DESKTOP"])
        .split('=')
        .nth(1)
        .unwrap_or_default()
        .trim()
        .to_owned()
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
fn focus_window_live_a_window_on_another_desktop_switches_the_desktop() {
    close_overview();
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let target = spawn(&mut launched, &bus, "target");
    let home = current_desktop();
    let away = if home == "0" { "1" } else { "0" };
    let started = Instant::now();
    while current_desktop() != away && started.elapsed() < Duration::from_secs(12) {
        let switched = tool("xdotool")
            .args(["set_desktop", away])
            .status()
            .expect("xdotool runs");
        assert!(switched.success());
        std::thread::sleep(Duration::from_millis(1_500));
    }
    assert_eq!(current_desktop(), away, "the desktop never switched away");
    std::thread::sleep(Duration::from_millis(1_000));
    focus(&bus, &target).expect("the window is activated across desktops");
    assert_eq!(current_desktop(), home);
    assert_eq!(active_pid(), Some(target.pid.get()));
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
fn focus_window_live_a_zenity_dialog_the_toolkit_never_reports_active_is_focused() {
    close_overview();
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let other = spawn(&mut launched, &bus, "other");
    assert!(wait_active(other.pid.get()));
    let pid = launched.spawn(
        "zenity",
        &["--info", "--title", "adl-zenity", "--text", "hello"],
    );
    let started = Instant::now();
    let dialog = loop {
        if let Some(window) = windows_of(&bus, pid).into_iter().next() {
            break window;
        }
        assert!(started.elapsed() < Duration::from_secs(20), "no dialog");
        std::thread::sleep(Duration::from_millis(200));
    };
    let blocker = spawn(&mut launched, &bus, "blocker");
    assert!(wait_active(blocker.pid.get()));
    focus(&bus, &dialog).expect("the dialog is active on the display with the focus in it");
    assert_eq!(active_pid(), Some(pid), "xprop agrees");
    let xid = xid_of(pid);
    let focused = tool("xdotool")
        .arg("getwindowfocus")
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
        .unwrap_or_default();
    let focus_xid = u32::from_str_radix(xid.trim_start_matches("0x"), 16).expect("an id");
    assert_eq!(
        focused,
        focus_xid.to_string(),
        "the input focus is in the dialog"
    );
}
