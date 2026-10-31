use agent_desktop_core::Rect;

use super::*;
use crate::system::hyprland::clients::client_workspace::ClientWorkspace;
use crate::system::hyprland::clients::monitor_workspaces::MonitorWorkspaces;

const ADDRESS: u64 = 0x10;

fn client(workspace: &str) -> HyprClient {
    HyprClient {
        address: ADDRESS,
        pid: 7,
        title: "Notes".into(),
        class: "app".into(),
        geometry: Rect {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 300.0,
        },
        workspace: ClientWorkspace {
            name: workspace.into(),
            monitor: Some(0),
        },
        shown: true,
    }
}

fn desktop(special: Option<&str>) -> DesktopState {
    DesktopState {
        active: None,
        monitors: vec![MonitorWorkspaces {
            monitor: "Virtual-1".into(),
            id: Some(0),
            active: "1".into(),
            special: special.map(str::to_owned),
        }],
        cursor: None,
    }
}

fn filling(entries: &[(u64, &str)]) -> Vec<(u64, String)> {
    entries
        .iter()
        .map(|(address, workspace)| (*address, (*workspace).to_owned()))
        .collect()
}

#[test]
fn a_shown_toplevel_on_the_active_workspace_is_current() {
    let none = filling(&[]);

    assert_eq!(
        composition_of(&client("1"), &desktop(None), Some(&none)),
        Composition::Current
    );
}

#[test]
fn a_toplevel_on_the_special_workspace_open_over_its_monitor_is_current() {
    let none = filling(&[]);

    assert_eq!(
        composition_of(
            &client("special:ad"),
            &desktop(Some("special:ad")),
            Some(&none)
        ),
        Composition::Current
    );
}

#[test]
fn a_toplevel_on_a_workspace_no_monitor_shows_is_stale() {
    let none = filling(&[]);

    assert_eq!(
        composition_of(&client("7"), &desktop(None), Some(&none)),
        Composition::Stale
    );
    assert_eq!(
        composition_of(&client("special:ad"), &desktop(None), Some(&none)),
        Composition::Stale
    );
}

#[test]
fn a_toplevel_under_an_open_special_workspace_is_stale() {
    let none = filling(&[]);

    assert_eq!(
        composition_of(&client("1"), &desktop(Some("special:ad")), Some(&none)),
        Composition::Stale
    );
}

#[test]
fn a_toplevel_behind_a_client_that_fills_its_workspace_is_stale() {
    let cover = filling(&[(0x20, "1")]);

    assert_eq!(
        composition_of(&client("1"), &desktop(None), Some(&cover)),
        Composition::Stale
    );
}

#[test]
fn a_client_filling_another_workspace_does_not_cover_it() {
    let elsewhere = filling(&[(0x20, "7")]);

    assert_eq!(
        composition_of(&client("1"), &desktop(None), Some(&elsewhere)),
        Composition::Current
    );
}

#[test]
fn a_toplevel_that_fills_its_own_workspace_is_current() {
    let itself = filling(&[(ADDRESS, "1")]);

    assert_eq!(
        composition_of(&client("1"), &desktop(None), Some(&itself)),
        Composition::Current
    );
}

#[test]
fn a_hidden_toplevel_is_stale() {
    let mut hidden = client("1");
    hidden.shown = false;
    let none = filling(&[]);

    assert_eq!(
        composition_of(&hidden, &desktop(None), Some(&none)),
        Composition::Stale
    );
}

#[test]
fn an_unknown_fullscreen_state_is_stale() {
    assert_eq!(
        composition_of(&client("1"), &desktop(None), None),
        Composition::Stale
    );
}
