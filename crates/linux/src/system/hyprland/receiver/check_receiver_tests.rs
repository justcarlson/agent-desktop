use agent_desktop_core::{Point, Rect};

use super::*;
use crate::system::hyprland::clients::client_workspace::ClientWorkspace;
use crate::system::hyprland::clients::desktop_state::DesktopState;
use crate::system::hyprland::clients::monitor_workspaces::MonitorWorkspaces;
use crate::system::hyprland::receiver::deny_all_overlays::DenyAllOverlays;
use crate::system::hyprland::receiver::layer_info::LayerInfo;
use crate::system::hyprland::receiver::stack_client::StackClient;
use crate::system::hyprland::receiver::stacking::Stacking;

#[path = "check_receiver_fullscreen_tests.rs"]
mod fullscreen_tests;

const TARGET: u64 = 0xa1;
const POINT: Point = Point { x: 162.0, y: 238.0 };

struct AcceptOwnCursor;

impl OverlayFilter for AcceptOwnCursor {
    fn is_own_cursor(&self, layer: &LayerInfo) -> bool {
        layer.namespace == "agent-desktop-cursor"
    }
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn workspace(name: &str) -> ClientWorkspace {
    ClientWorkspace {
        name: name.to_owned(),
        monitor: Some(0),
    }
}

fn client(address: u64, at: Rect, ws: &str) -> StackClient {
    StackClient {
        address,
        rect: at,
        workspace: workspace(ws),
        stacking: Stacking::default(),
        shown: true,
    }
}

fn layer(namespace: &str, level: u8, at: Rect) -> LayerInfo {
    LayerInfo {
        namespace: namespace.to_owned(),
        pid: Some(724),
        rect: at,
        level,
        output: "Virtual-1".to_owned(),
    }
}

fn desktop(special: Option<&str>) -> DesktopState {
    DesktopState {
        active: Some(TARGET),
        monitors: vec![MonitorWorkspaces {
            monitor: "Virtual-1".to_owned(),
            id: Some(0),
            active: "1".to_owned(),
            special: special.map(str::to_owned),
        }],
        cursor: None,
    }
}

fn stack() -> ReceiverStack {
    ReceiverStack {
        clients: vec![client(TARGET, rect(12.0, 38.0, 1256.0, 750.0), "1")],
        layers: vec![layer(
            "omarchy-background",
            0,
            rect(0.0, 0.0, 1280.0, 800.0),
        )],
        desktop: desktop(None),
    }
}

fn check(stack: &ReceiverStack) -> Result<(), ReceiverRefusal> {
    check_receiver(stack, TARGET, &POINT, &DenyAllOverlays)
}

#[test]
fn a_point_inside_the_shown_target_with_no_cover_passes() {
    assert_eq!(check(&stack()), Ok(()));
}

#[test]
fn a_missing_target_refuses_as_missing_evidence() {
    let mut stack = stack();
    stack.clients.clear();
    assert!(matches!(
        check(&stack),
        Err(ReceiverRefusal::EvidenceMissing { .. })
    ));
}

#[test]
fn a_point_outside_the_target_refuses() {
    let outside = Point { x: 5.0, y: 5.0 };
    assert_eq!(
        check_receiver(&stack(), TARGET, &outside, &DenyAllOverlays),
        Err(ReceiverRefusal::PointOutsideTarget)
    );
}

#[test]
fn a_hidden_target_or_a_special_in_front_refuses() {
    let mut hidden = stack();
    hidden.clients[0].shown = false;
    assert_eq!(check(&hidden), Err(ReceiverRefusal::TargetNotShown));
    let mut special = stack();
    special.desktop = desktop(Some("special:magic"));
    assert_eq!(check(&special), Err(ReceiverRefusal::TargetNotShown));
}

#[test]
fn a_top_or_overlay_layer_over_the_point_refuses() {
    for level in [2, 3] {
        let mut stack = stack();
        stack
            .layers
            .push(layer("omarchy-bar", level, rect(0.0, 0.0, 1280.0, 800.0)));
        assert_eq!(
            check(&stack),
            Err(ReceiverRefusal::LayerCover {
                namespace: "omarchy-bar".to_owned(),
                level
            })
        );
    }
}

#[test]
fn a_layer_beside_the_point_or_below_windows_passes() {
    let mut stack = stack();
    stack
        .layers
        .push(layer("omarchy-dock", 2, rect(0.0, 593.0, 1280.0, 207.0)));
    stack
        .layers
        .push(layer("wallpaper", 1, rect(0.0, 0.0, 1280.0, 800.0)));
    assert_eq!(check(&stack), Ok(()));
}

#[test]
fn an_agent_desktop_cursor_layer_refuses_with_the_default_filter() {
    let mut stack = stack();
    stack.layers.push(layer(
        "agent-desktop-cursor",
        3,
        rect(150.0, 230.0, 24.0, 24.0),
    ));
    assert_eq!(
        check(&stack),
        Err(ReceiverRefusal::LayerCover {
            namespace: "agent-desktop-cursor".to_owned(),
            level: 3
        })
    );
    assert_eq!(
        check_receiver(&stack, TARGET, &POINT, &AcceptOwnCursor),
        Ok(())
    );
}

#[test]
fn a_floating_pinned_or_fullscreen_window_over_the_point_refuses() {
    let over = rect(100.0, 200.0, 300.0, 200.0);
    let covers: [fn(&mut StackClient); 3] = [
        |c| c.stacking.floating = true,
        |c| c.stacking.pinned = true,
        |c| c.stacking.fullscreen = true,
    ];
    for cover in covers {
        let mut stack = stack();
        let mut other = client(0xb2, over, "1");
        cover(&mut other);
        stack.clients.push(other);
        assert_eq!(
            check(&stack),
            Err(ReceiverRefusal::WindowCover { address: 0xb2 })
        );
    }
}

#[test]
fn a_pinned_window_from_another_workspace_still_covers() {
    let mut stack = stack();
    let mut other = client(0xb2, rect(100.0, 200.0, 300.0, 200.0), "9");
    other.stacking.pinned = true;
    stack.clients.push(other);
    assert_eq!(
        check(&stack),
        Err(ReceiverRefusal::WindowCover { address: 0xb2 })
    );
}

#[test]
fn a_hidden_or_off_view_floating_window_does_not_cover() {
    let over = rect(100.0, 200.0, 300.0, 200.0);
    let mut stack = stack();
    let mut hidden = client(0xb2, over, "1");
    hidden.stacking.floating = true;
    hidden.shown = false;
    let mut elsewhere = client(0xc3, over, "4");
    elsewhere.stacking.floating = true;
    stack.clients.extend([hidden, elsewhere]);
    assert_eq!(check(&stack), Ok(()));
}

#[test]
fn the_refusal_error_is_not_delivered_with_its_cause() {
    let error = ReceiverRefusal::LayerCover {
        namespace: "omarchy-bar".to_owned(),
        level: 2,
    }
    .to_error();
    assert_eq!(
        error.disposition,
        agent_desktop_core::DeliverySemantics::not_delivered()
    );
    let details = error.details.unwrap_or_default();
    assert_eq!(details["cause"], "layer_cover");
    assert_eq!(details["physical_delivery_started"], false);
    assert_eq!(details["layer_namespace"], "omarchy-bar");
}

fn floating(address: u64) -> StackClient {
    let mut other = client(address, rect(100.0, 200.0, 300.0, 200.0), "1");
    other.stacking.floating = true;
    other.stacking.over_fullscreen = Some(false);
    other
}

fn floating_target(stack: &mut ReceiverStack) {
    stack.clients[0].stacking.floating = true;
    stack.clients[0].stacking.over_fullscreen = Some(false);
}

#[test]
fn a_floating_window_below_a_floating_target_does_not_cover() {
    let mut stack = stack();
    floating_target(&mut stack);
    stack.clients.insert(0, floating(0xb2));
    assert_eq!(check(&stack), Ok(()));
}

#[test]
fn a_floating_window_above_a_floating_target_covers() {
    let mut stack = stack();
    floating_target(&mut stack);
    stack.clients.push(floating(0xb2));
    assert_eq!(
        check(&stack),
        Err(ReceiverRefusal::WindowCover { address: 0xb2 })
    );
}

#[test]
fn a_floating_window_listed_before_a_tiled_target_still_covers() {
    let mut stack = stack();
    stack.clients.insert(0, floating(0xb2));
    assert_eq!(
        check(&stack),
        Err(ReceiverRefusal::WindowCover { address: 0xb2 })
    );
}

#[test]
fn a_pinned_window_listed_before_a_floating_target_still_covers() {
    let mut stack = stack();
    floating_target(&mut stack);
    let mut pinned = floating(0xb2);
    pinned.stacking.pinned = true;
    stack.clients.insert(0, pinned);
    assert_eq!(
        check(&stack),
        Err(ReceiverRefusal::WindowCover { address: 0xb2 })
    );
}

#[test]
fn a_floating_window_below_a_pinned_target_does_not_cover() {
    let mut stack = stack();
    floating_target(&mut stack);
    stack.clients[0].stacking.pinned = true;
    stack.clients.push(floating(0xb2));
    assert_eq!(check(&stack), Ok(()));
}
