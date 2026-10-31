use agent_desktop_core::Rect;

use super::*;
use crate::system::hyprland::clients::client_workspace::ClientWorkspace;

fn client(address: u64, pid: u32) -> HyprClient {
    HyprClient {
        address,
        pid,
        title: "t".into(),
        class: "app".into(),
        geometry: Rect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        },
        workspace: ClientWorkspace {
            name: "1".into(),
            monitor: Some(0),
        },
        shown: true,
    }
}

fn window(id: u32, address: u64, title: &str, child: bool) -> WlrWindow {
    WlrWindow {
        id,
        address,
        title: title.into(),
        activated: false,
        child,
    }
}

fn desktop() -> (Vec<WlrWindow>, Vec<HyprClient>) {
    (
        vec![
            window(3, 0x10, "Notes", false),
            window(4, 0x20, "foot", false),
            window(5, 0x30, "Save changes?", true),
        ],
        vec![client(0x10, 7), client(0x20, 9), client(0x30, 7)],
    )
}

#[test]
fn only_the_processs_toplevels_are_listed_and_a_child_counts_as_a_dialog() {
    let (windows, clients) = desktop();

    let owned = owned_toplevels(&windows, &clients, 7);

    assert_eq!(
        owned,
        vec![
            Toplevel {
                id: 3,
                title: "Notes".into(),
                kind: WindowKind::Normal,
                transient: false,
            },
            Toplevel {
                id: 5,
                title: "Save changes?".into(),
                kind: WindowKind::Normal,
                transient: true,
            },
        ]
    );
}

#[test]
fn a_close_goes_only_to_a_toplevel_the_process_still_owns() {
    let (windows, mut clients) = desktop();

    assert_eq!(check_owner(3, &windows, &clients, 7), Ok(()));
    assert_eq!(
        check_owner(4, &windows, &clients, 7),
        Err(ControlError::StaleWindow)
    );
    assert_eq!(
        check_owner(8, &windows, &clients, 7),
        Err(ControlError::BadWindow)
    );
    clients.remove(0);
    assert_eq!(
        check_owner(3, &windows, &clients, 7),
        Err(ControlError::BadWindow)
    );
}
