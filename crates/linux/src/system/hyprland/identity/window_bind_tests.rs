use agent_desktop_core::{ErrorCode, ProcessId, Rect, WindowState};

use super::*;
use crate::system::hyprland::clients::client_workspace::ClientWorkspace;
use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::hyprland::identity::compositor_window_id::encode;
use crate::system::hyprland::toplevel::toplevel_view::ToplevelView;

const LIVE: Option<&str> = Some("boot:100");
const COMPOSITOR: (u32, u64) = (50, 60);

fn client(address: u64, title: &str) -> HyprClient {
    HyprClient {
        address,
        pid: 7,
        title: title.into(),
        class: "app".into(),
        geometry: Rect {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 300.0,
        },
        workspace: ClientWorkspace {
            name: "special:hid".into(),
            monitor: None,
        },
        shown: false,
    }
}

fn view(identifier: &str, address: u64) -> ToplevelView {
    ToplevelView {
        identifier: identifier.into(),
        address,
    }
}

fn window(id: String, title: &str) -> WindowInfo {
    WindowInfo {
        id,
        title: title.into(),
        app: "app".into(),
        pid: ProcessId::new(7),
        process_instance: Some("boot:100".into()),
        bounds: None,
        state: WindowState::default(),
    }
}

fn bind(win: &WindowInfo) -> Result<BoundToplevel, AdapterError> {
    let clients = [client(0x10, "Notes"), client(0x20, "Mail")];
    let toplevels = [view("a", 0x10), view("b", 0x20)];
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };
    bind_window(win, inventory, LIVE, COMPOSITOR)
}

#[test]
fn a_frame_binds_to_its_client_and_that_clients_own_toplevel() {
    let bound = bind(&window("w-1.2-3".into(), "Mail")).unwrap();
    assert_eq!(bound.client.address, 0x20);
    assert_eq!(bound.toplevel, view("b", 0x20));
}

#[test]
fn a_compositor_window_binds_to_the_toplevel_its_identifier_names() {
    let id = encode(COMPOSITOR.0, COMPOSITOR.1, "a").unwrap();
    let bound = bind(&window(id, "renamed")).unwrap();
    assert_eq!(bound.client.address, 0x10);
    assert_eq!(bound.toplevel, view("a", 0x10));
}

#[test]
fn a_window_that_matches_no_toplevel_binds_nothing() {
    assert_eq!(
        bind(&window("w-1.2-3".into(), "Gone")).unwrap_err().code,
        ErrorCode::WindowNotFound
    );
    let id = encode(COMPOSITOR.0, COMPOSITOR.1, "missing").unwrap();
    assert_eq!(
        bind(&window(id, "x")).unwrap_err().code,
        ErrorCode::WindowNotFound
    );
}
