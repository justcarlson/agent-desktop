use agent_desktop_core::{DisplayInfo, Rect};

use super::*;
use crate::system::hyprland::clients::client_workspace::ClientWorkspace;
use crate::system::hyprland::clients::desktop_state::DesktopState;
use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::hyprland::clients::monitor_workspaces::MonitorWorkspaces;

fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn display(id: &str, bounds: Rect, scale: f64) -> DisplayInfo {
    DisplayInfo {
        id: id.to_owned(),
        bounds,
        is_primary: false,
        scale,
    }
}

fn displays() -> Vec<DisplayInfo> {
    vec![
        display("DP-1", rect(0.0, 0.0, 1280.0, 800.0), 1.0),
        display("AD-E2E-1", rect(-1024.0, -640.0, 1024.0, 640.0), 1.25),
    ]
}

fn client(at: (f64, f64), workspace: &str, shown: bool) -> HyprClient {
    HyprClient {
        address: 0xa1,
        pid: 42,
        title: "zoo".to_owned(),
        class: "zoo".to_owned(),
        geometry: rect(at.0, at.1, 600.0, 400.0),
        workspace: ClientWorkspace {
            name: workspace.to_owned(),
            monitor: Some(1),
        },
        shown,
    }
}

fn desktop(active: &str, special: Option<&str>) -> DesktopState {
    DesktopState {
        active: Some(0xa1),
        monitors: vec![
            MonitorWorkspaces {
                monitor: "DP-1".to_owned(),
                id: Some(0),
                active: "1".to_owned(),
                special: None,
            },
            MonitorWorkspaces {
                monitor: "AD-E2E-1".to_owned(),
                id: Some(1),
                active: active.to_owned(),
                special: special.map(str::to_owned),
            },
        ],
        cursor: None,
    }
}

fn place(client: &HyprClient, desktop: &DesktopState, window_rect: Rect) -> Placement {
    let displays = displays();
    layout_rect(
        &BoundLayout {
            client,
            displays: &displays,
            desktop,
        },
        &window_rect,
    )
}

#[test]
fn a_negative_origin_adds_the_client_origin_to_the_window_rect() {
    let placed = place(
        &client((-1012.0, -602.0), "7", true),
        &desktop("7", None),
        rect(150.0, 200.0, 20.0, 10.0),
    );
    assert_eq!(placed.rect, rect(-862.0, -402.0, 20.0, 10.0));
    assert_eq!(placed.output.as_deref(), Some("AD-E2E-1"));
    assert!(placed.shown);
}

#[test]
fn a_fractional_scale_does_not_rescale_logical_pixels() {
    let placed = place(
        &client((-1012.0, -602.0), "7", true),
        &desktop("7", None),
        rect(10.5, 20.25, 7.5, 3.0),
    );
    assert_eq!(placed.rect, rect(-1001.5, -581.75, 7.5, 3.0));
}

#[test]
fn a_centre_on_no_output_is_not_shown() {
    let placed = place(
        &client((2000.0, 0.0), "1", true),
        &desktop("7", None),
        rect(10.0, 10.0, 20.0, 20.0),
    );
    assert_eq!(placed.output, None);
    assert!(!placed.shown);
}

#[test]
fn a_hidden_workspace_or_a_special_in_front_is_not_shown() {
    let window = rect(150.0, 200.0, 20.0, 10.0);
    let at = (-1012.0, -602.0);
    assert!(!place(&client(at, "8", true), &desktop("7", None), window).shown);
    assert!(
        !place(
            &client(at, "7", true),
            &desktop("7", Some("special:s")),
            window
        )
        .shown
    );
    assert!(!place(&client(at, "7", false), &desktop("7", None), window).shown);
}
