use agent_desktop_core::Rect;

use super::*;
use crate::system::hyprland::clients::client_workspace::ClientWorkspace;
use crate::system::hyprland::clients::desktop_state::DesktopState;
use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::hyprland::clients::monitor_workspaces::MonitorWorkspaces;
use crate::system::hyprland::toplevel::toplevel_view::ToplevelView;

fn client(address: u64, pid: u32, class: &str, shown: bool) -> HyprClient {
    HyprClient {
        address,
        pid,
        title: format!("{class} window"),
        class: class.into(),
        geometry: Rect {
            x: 10.0,
            y: 20.0,
            width: 300.0,
            height: 200.0,
        },
        workspace: ClientWorkspace {
            name: "1".into(),
            monitor: Some(0),
        },
        shown,
    }
}

fn view(identifier: &str, address: u64) -> ToplevelView {
    ToplevelView {
        identifier: identifier.into(),
        address,
    }
}

fn desktop(active: Option<u64>, special: Option<&str>) -> DesktopState {
    DesktopState {
        active,
        monitors: vec![MonitorWorkspaces {
            id: Some(0),
            active: "1".into(),
            special: special.map(str::to_owned),
        }],
    }
}

fn instance(pid: u32) -> Option<String> {
    (pid != 99).then(|| format!("boot:{pid}"))
}

#[test]
fn toplevels_of_processes_without_an_accessibility_tree_are_listed_by_compositor_id() {
    let clients = [
        client(0x10, 7, "foot", true),
        client(0x20, 8, "org.gnome.Calculator", true),
        client(0x30, 7, "foot", false),
    ];
    let toplevels = [view("a1", 0x10), view("b2", 0x20), view("c3", 0x30)];
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };

    let windows = compositor_windows(
        inventory,
        &desktop(Some(0x30), None),
        (50, 60),
        |client| client.pid == 8,
        instance,
    );

    let ids: Vec<&str> = windows.iter().map(|window| window.id.as_str()).collect();
    assert_eq!(ids, vec!["w-hypr-50-60-a1", "w-hypr-50-60-c3"]);
    let first = &windows[0];
    assert_eq!(first.title, "foot window");
    assert_eq!(first.app, "foot");
    assert_eq!(first.pid.get(), 7);
    assert_eq!(first.process_instance.as_deref(), Some("boot:7"));
    assert_eq!(first.bounds.map(|bounds| bounds.width), Some(300.0));
    assert!(!first.state.accessible);
    assert!(!first.state.is_focused);
    assert_eq!(first.state.visible, Some(true));
    assert!(windows[1].state.is_focused);
    assert_eq!(windows[1].state.visible, Some(false));
}

#[test]
fn toplevels_without_a_client_a_live_owner_or_a_plain_identifier_are_not_listed() {
    let clients = [
        client(0x10, 99, "gone", true),
        client(0x20, 7, "foot", true),
    ];
    let toplevels = [view("a1", 0x10), view("bad-id", 0x20), view("orphan", 0x40)];
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };

    let windows = compositor_windows(
        inventory,
        &desktop(None, None),
        (50, 60),
        |_| false,
        instance,
    );

    assert!(windows.is_empty());
}

#[test]
fn a_toplevel_its_registered_process_exposes_no_frame_for_is_listed() {
    let clients = [
        client(0x10, 8, "electron", true),
        client(0x20, 8, "dialog", true),
    ];
    let toplevels = [view("a1", 0x10), view("b2", 0x20)];
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };

    let windows = compositor_windows(
        inventory,
        &desktop(None, None),
        (50, 60),
        |client| client.address == 0x20,
        instance,
    );

    let ids: Vec<&str> = windows.iter().map(|window| window.id.as_str()).collect();
    assert_eq!(ids, vec!["w-hypr-50-60-a1"]);
}

#[test]
fn the_active_window_is_not_focused_while_its_workspace_is_out_of_view() {
    let mut hidden = client(0x10, 7, "foot", false);
    hidden.workspace.name = "special:scratch".into();
    let clients = [hidden, client(0x20, 7, "foot", true)];
    let toplevels = [view("a1", 0x10), view("b2", 0x20)];
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };

    let on_hidden_special = compositor_windows(
        inventory,
        &desktop(Some(0x10), None),
        (50, 60),
        |_| false,
        instance,
    );
    let under_open_special = compositor_windows(
        inventory,
        &desktop(Some(0x20), Some("special:scratch")),
        (50, 60),
        |_| false,
        instance,
    );

    assert!(
        on_hidden_special
            .iter()
            .all(|window| !window.state.is_focused)
    );
    assert!(
        under_open_special
            .iter()
            .all(|window| !window.state.is_focused)
    );
}
