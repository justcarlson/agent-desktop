use agent_desktop_core::{ProcessId, WindowInfo, WindowState};

use crate::system::shell::fake_tree::{BUS, FakeTree, PANEL, PID, WINDOW, at, element};
use crate::system::shell::identity::{
    ShellRoot, content_window, is_shell_comm, is_shell_owner, pick_shell_app,
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
fn only_the_shell_command_name_is_the_shell() {
    assert!(is_shell_comm("gnome-shell"));
    assert!(!is_shell_comm("gnome-shell-cal"));
    assert!(!is_shell_comm("gjs"));
}

#[test]
fn the_content_window_is_the_child_with_the_window_role() {
    let mut tree = FakeTree::new();
    tree.add(None, "/root", 75, "gnome-shell", at(0, 0, 0, 0));
    tree.add(Some("/root"), "/decor", PANEL, "", at(0, 0, 1, 1));
    tree.add(Some("/root"), "/w", WINDOW, "", at(0, 54, 87, 55));
    let window = content_window(&tree, &element("/root")).expect("a window child");
    assert_eq!(window.path(), "/w");
}

#[test]
fn an_app_with_no_window_child_has_no_content_window() {
    let mut tree = FakeTree::new();
    tree.add(None, "/root", 75, "gnome-shell", at(0, 0, 0, 0));
    tree.add(Some("/root"), "/decor", PANEL, "", at(0, 0, 1, 1));
    assert!(content_window(&tree, &element("/root")).is_none());
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
}
