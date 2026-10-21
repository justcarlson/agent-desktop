use agent_desktop_core::{ProcessId, WindowInfo, WindowState};

use crate::system::shell::fake_tree::{BUS, FakeTree, PANEL, PID, WINDOW, at, element};
use crate::system::shell::identity::{
    ShellRoot, content_window, is_shell_comm, is_shell_owner, pick_shell_app, retries_after,
    retry_until, window_missing,
};
use crate::system::window_id;
use crate::tree::AtspiElement;

fn app(bus: &str, pid: u32) -> (AtspiElement, u32) {
    (AtspiElement::new(bus, "/root", pid), pid)
}

#[test]
fn the_shell_app_is_the_one_owned_by_the_shell_process() {
    let apps = [app(":1.2", 10), app(":1.3", 20), app(":1.4", 30)];
    let picked = pick_shell_app(&apps, 20).expect("pid 20 owns an app");
    assert_eq!(picked.bus_name(), ":1.3");
    assert!(pick_shell_app(&apps, 99).is_none());
}

#[test]
fn the_content_window_is_the_child_with_the_window_role() {
    let mut tree = FakeTree::new();
    tree.add(None, "/root", 75, "gnome-shell", at(0, 0, 0, 0));
    tree.add(Some("/root"), "/decor", PANEL, "", at(0, 0, 1, 1));
    tree.add(Some("/root"), "/w", WINDOW, "", at(0, 54, 87, 55));
    let window = content_window(&tree, &element("/root"))
        .expect("a readable app")
        .expect("a window child");
    assert_eq!(window.path(), "/w");
}

#[test]
fn an_app_with_no_window_child_has_no_content_window() {
    let mut tree = FakeTree::new();
    tree.add(None, "/root", 75, "gnome-shell", at(0, 0, 0, 0));
    tree.add(Some("/root"), "/decor", PANEL, "", at(0, 0, 1, 1));
    assert!(
        content_window(&tree, &element("/root"))
            .expect("a readable app")
            .is_none()
    );
}

#[test]
fn a_destroyed_sibling_does_not_hide_the_live_content_window() {
    let mut tree = FakeTree::new();
    tree.add(None, "/root", 75, "gnome-shell", at(0, 0, 0, 0));
    tree.add(Some("/root"), "/decor", PANEL, "", at(0, 0, 1, 1));
    tree.add(Some("/root"), "/w", WINDOW, "", at(0, 54, 87, 55));
    tree.vanish("/decor");
    let window = content_window(&tree, &element("/root"))
        .expect("the live sibling is still readable")
        .expect("the content window");
    assert_eq!(window.path(), "/w");
}

fn window(bus: &str, pid: u32) -> WindowInfo {
    WindowInfo {
        id: window_id::encode(bus, "/org/a11y/atspi/accessible/5").expect("an id"),
        title: String::new(),
        app: "x".to_owned(),
        pid: ProcessId::from(pid),
        process_instance: None,
        bounds: None,
        state: WindowState::default(),
    }
}

#[test]
fn a_window_of_the_remembered_shell_is_a_shell_window_and_another_never_is() {
    crate::system::shell::identity_remember_for_tests(&ShellRoot {
        bus_name: ":77.77".to_owned(),
        path: "/w".to_owned(),
        pid: PID,
    });
    assert!(is_shell_owner(":77.77", u32::MAX));
    assert!(crate::system::shell::is_shell_window(&window(
        ":77.77",
        u32::MAX
    )));
    assert!(!crate::system::shell::is_shell_window(&window(
        ":77.78",
        u32::MAX - 1
    )));
    assert!(!is_shell_owner(BUS, std::process::id()));
    assert!(is_shell_comm("gnome-shell"));
    assert!(!is_shell_comm("gnome-shell-cal"));
    assert!(!is_shell_comm("gjs"));
}

fn deadline() -> agent_desktop_core::Deadline {
    agent_desktop_core::Deadline::after(10_000).expect("a deadline")
}

#[test]
fn a_window_that_comes_back_within_the_retries_is_found() {
    let mut misses = 3;
    let mut pauses = 0;
    let found = retry_until(10, deadline(), &mut |_| pauses += 1, || {
        if misses == 0 {
            Some("window")
        } else {
            misses -= 1;
            None
        }
    });
    assert_eq!(found, Some("window"));
    assert_eq!(pauses, 3);
}

#[test]
fn a_window_that_never_comes_back_is_none_after_the_bounded_retries() {
    let mut pauses = 0;
    let found: Option<()> = retry_until(10, deadline(), &mut |_| pauses += 1, || None);
    assert!(found.is_none());
    assert_eq!(pauses, 10);
}

#[test]
fn the_retries_stop_at_a_spent_deadline() {
    let spent = agent_desktop_core::Deadline::after(1).expect("a deadline");
    std::thread::sleep(std::time::Duration::from_millis(10));
    let mut pauses = 0;
    let found: Option<()> = retry_until(10, spent, &mut |_| pauses += 1, || None);
    assert!(found.is_none());
    assert_eq!(pauses, 0);
}

#[test]
fn a_missing_shell_window_is_named_apart_from_an_absent_shell_and_is_safe_to_retry() {
    let error = window_missing();
    assert_eq!(error.code, agent_desktop_core::ErrorCode::AppNotFound);
    let details = error.details.expect("details");
    assert_eq!(details["shell_tree"], "window_missing");
    assert_eq!(details["retryable"], true);
    assert_eq!(
        error.disposition,
        agent_desktop_core::DeliverySemantics::not_delivered()
    );
}

#[test]
fn a_lookup_in_the_same_outage_probes_once_and_a_fresh_one_retries() {
    let now = std::time::Instant::now();
    assert_eq!(retries_after(Some(now), now), 0);
    assert_eq!(retries_after(None, now), 10);
    let long_ago = now - std::time::Duration::from_millis(5_000);
    assert_eq!(retries_after(Some(long_ago), now), 10);
}
