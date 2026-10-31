use std::collections::HashSet;

use agent_desktop_core::Rect;

use super::*;
use crate::system::hyprland::clients::client_workspace::ClientWorkspace;
use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::wayland::toplevel::toplevel_view::ToplevelView;

fn client(address: u64, pid: u32, title: &str, size: (f64, f64)) -> HyprClient {
    HyprClient {
        address,
        pid,
        title: title.into(),
        class: "app".into(),
        geometry: Rect {
            x: 0.0,
            y: 0.0,
            width: size.0,
            height: size.1,
        },
        workspace: ClientWorkspace {
            name: "1".into(),
            monitor: Some(0),
        },
        shown: true,
    }
}

fn view(identifier: &str, address: u64) -> ToplevelView {
    ToplevelView {
        identifier: identifier.into(),
        address,
        activated: false,
        wlr_index: Some(0),
    }
}

fn frame(pid: u32, title: &str, size: Option<(f64, f64)>) -> WindowEvidence {
    WindowEvidence {
        pid,
        instance: Some("boot:1".into()),
        title: title.into(),
        size,
    }
}

fn clients() -> [HyprClient; 5] {
    [
        client(0x10, 7, "Notes", (400.0, 300.0)),
        client(0x20, 7, "Notes", (800.0, 600.0)),
        client(0x30, 7, "Preferences", (400.0, 300.0)),
        client(0x40, 8, "Slack", (400.0, 300.0)),
        client(0x50, 9, "Skipped", (400.0, 300.0)),
    ]
}

fn toplevels() -> [ToplevelView; 5] {
    [
        view("a", 0x10),
        view("b", 0x20),
        view("c", 0x30),
        view("d", 0x40),
        view("e", 0x50),
    ]
}

#[test]
fn a_frame_hides_only_the_toplevel_it_binds_to() {
    let (clients, toplevels) = (clients(), toplevels());
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };
    let frames = AccessibleFrames {
        frames: vec![frame(7, "Notes", Some((400.0, 300.0)))],
        read: HashSet::from([7, 8]),
        registered: HashSet::from([7, 8, 9]),
    };

    assert_eq!(frames.represented(inventory), HashSet::from([0x10, 0x50]));
}

#[test]
fn a_frame_that_cannot_tell_its_twins_apart_hides_every_twin() {
    let (clients, toplevels) = (clients(), toplevels());
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };
    let mut unknown = frame(7, "Preferences", None);
    unknown.instance = None;
    let frames = AccessibleFrames {
        frames: vec![frame(7, "Notes", Some((1.0, 1.0))), unknown],
        read: HashSet::from([7]),
        registered: HashSet::from([7]),
    };

    assert_eq!(
        frames.represented(inventory),
        HashSet::from([0x10, 0x20, 0x30])
    );
}
