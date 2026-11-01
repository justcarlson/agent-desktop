use agent_desktop_core::{HitTestResult, Point, Rect};

use super::*;
use crate::system::hyprland::clients::client_workspace::ClientWorkspace;
use crate::system::hyprland::clients::desktop_state::DesktopState;
use crate::system::hyprland::clients::monitor_workspaces::MonitorWorkspaces;
use crate::system::hyprland::receiver::layer_info::LayerInfo;
use crate::system::hyprland::receiver::stack_client::StackClient;

const TARGET: u64 = 0xa1;
const POINT: Point = Point { x: 162.0, y: 238.0 };

fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn client(address: u64, at: Rect) -> StackClient {
    StackClient {
        address,
        rect: at,
        workspace: ClientWorkspace {
            name: "1".to_owned(),
            monitor: Some(0),
        },
        floating: false,
        pinned: false,
        fullscreen: false,
        shown: true,
    }
}

fn stack() -> ReceiverStack {
    ReceiverStack {
        clients: vec![client(TARGET, rect(12.0, 38.0, 1256.0, 750.0))],
        layers: Vec::new(),
        desktop: DesktopState {
            active: Some(TARGET),
            monitors: vec![MonitorWorkspaces {
                monitor: "Virtual-1".to_owned(),
                id: Some(0),
                active: "1".to_owned(),
                special: None,
            }],
            cursor: None,
        },
    }
}

#[test]
fn a_point_on_the_uncovered_target_reaches_it() {
    assert_eq!(
        hit_of(&stack(), TARGET, &POINT),
        HitTestResult::ReachesTarget
    );
}

#[test]
fn a_floating_window_over_the_point_intercepts_with_its_rectangle() {
    let mut stack = stack();
    let over = rect(100.0, 200.0, 300.0, 200.0);
    let mut cover = client(0xb2, over);
    cover.floating = true;
    stack.clients.push(cover);
    assert_eq!(
        hit_of(&stack, TARGET, &POINT),
        HitTestResult::InterceptedBy {
            role: Some("window".to_owned()),
            name: None,
            bounds: Some(over),
        }
    );
}

#[test]
fn a_layer_over_the_point_intercepts_with_its_namespace() {
    let mut stack = stack();
    let over = rect(0.0, 0.0, 1280.0, 800.0);
    stack.layers.push(LayerInfo {
        namespace: "waybar".to_owned(),
        pid: Some(724),
        rect: over,
        level: 2,
        output: "Virtual-1".to_owned(),
    });
    assert_eq!(
        hit_of(&stack, TARGET, &POINT),
        HitTestResult::InterceptedBy {
            role: Some("layer".to_owned()),
            name: Some("waybar".to_owned()),
            bounds: Some(over),
        }
    );
}

#[test]
fn a_missing_hidden_or_missed_target_is_unknown() {
    assert_eq!(hit_of(&stack(), 0xdead, &POINT), HitTestResult::Unknown);
    let mut hidden = stack();
    hidden.clients[0].shown = false;
    assert_eq!(hit_of(&hidden, TARGET, &POINT), HitTestResult::Unknown);
    let outside = Point { x: 2.0, y: 2.0 };
    assert_eq!(hit_of(&stack(), TARGET, &outside), HitTestResult::Unknown);
}
