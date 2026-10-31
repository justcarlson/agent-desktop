use agent_desktop_core::{
    DeliverySemantics, ErrorCode, FocusProof, FocusSideEffect, InteractionPolicy, Point, ProcessId,
    Rect, WindowInfo, WindowState,
};

use super::*;
use crate::system::hyprland::clients::client_workspace::ClientWorkspace;
use crate::system::hyprland::clients::desktop_state::DesktopState;
use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::hyprland::clients::monitor_workspaces::MonitorWorkspaces;
use crate::system::hyprland::focus::fake_snapshot::FakeSnapshot;
use crate::system::hyprland::focus::hypr_world_fake::FakeHyprWorld;
use crate::system::wayland::toplevel::toplevel_view::ToplevelView;

pub(super) const TARGET: u64 = 0x10;
pub(super) const BYSTANDER: u64 = 0x30;

fn client(address: u64, pid: u32, title: &str, workspace: &str) -> HyprClient {
    HyprClient {
        address,
        pid,
        title: title.into(),
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

pub(super) fn snapshot(
    active: u64,
    workspace: &str,
    special: Option<&str>,
    frame_active: bool,
) -> FakeSnapshot {
    let view = |identifier: &str, address: u64| ToplevelView {
        identifier: identifier.into(),
        address,
        activated: address == active,
        wlr_index: Some(0),
    };
    FakeSnapshot {
        desktop: DesktopState {
            active: Some(active),
            monitors: vec![MonitorWorkspaces {
                monitor: "Virtual-1".into(),
                id: Some(0),
                active: workspace.into(),
                special: special.map(Into::into),
            }],
            cursor: None,
        },
        toplevels: vec![view("a", TARGET), view("b", BYSTANDER), view("c", 0x20)],
        frame_active,
    }
}

pub(super) fn world(
    target_workspace: &str,
    before: FakeSnapshot,
    after: Option<FakeSnapshot>,
) -> FakeHyprWorld {
    FakeHyprWorld {
        clients: vec![
            client(TARGET, 7, "Notes", target_workspace),
            client(BYSTANDER, 9, "foot", "1"),
            client(0x20, 7, "Notes — other", target_workspace),
        ],
        before,
        after,
        lag: 0,
        budget: 5,
        live: Some("boot:100".into()),
        log: Vec::new(),
    }
}

pub(super) fn frame_window() -> WindowInfo {
    WindowInfo {
        id: "w-1.2-3".into(),
        title: "Notes".into(),
        app: "app".into(),
        pid: ProcessId::new(7),
        process_instance: Some("boot:100".into()),
        bounds: Some(Rect {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 300.0,
        }),
        state: WindowState::default(),
    }
}

pub(super) fn compositor_window(identifier: &str) -> WindowInfo {
    let mut window = frame_window();
    window.id = format!("w-hypr-50-60-{identifier}");
    window.title = "foot".into();
    window.state.accessible = false;
    window
}

pub(super) fn headless() -> InteractionPolicy {
    InteractionPolicy::headless()
}

pub(super) fn headed() -> InteractionPolicy {
    InteractionPolicy::headed()
}

#[test]
fn an_already_focused_frame_is_proven_without_activation_in_either_mode() {
    for policy in [headless(), headed()] {
        let mut fake = world("1", snapshot(TARGET, "1", None, true), None);

        let outcome = focus_reported(&mut fake, &frame_window(), policy).expect("focused");

        assert!(!outcome.activation.activated);
        assert_eq!(
            outcome.activation.proof,
            FocusProof::CompositorAndAccessibility
        );
        assert!(outcome.activation.side_effects.is_empty());
        assert!(outcome.window.state.is_focused);
        assert!(fake.activations().is_empty());
    }
}

#[test]
fn an_already_focused_compositor_window_is_proven_by_the_compositor_alone() {
    let mut fake = world("1", snapshot(TARGET, "1", None, false), None);

    let outcome = focus_reported(&mut fake, &compositor_window("a"), headless()).expect("focused");

    assert_eq!(outcome.activation.proof, FocusProof::CompositorOnly);
    assert!(!outcome.window.state.accessible);
    assert!(!fake.log.contains(&"frame".to_string()));
}

#[test]
fn headless_focus_of_an_unfocused_window_refuses_without_activating() {
    for policy in [headless(), InteractionPolicy::focus_fallback()] {
        let mut fake = world("2", snapshot(BYSTANDER, "1", None, false), None);

        let error = focus_reported(&mut fake, &frame_window(), policy).unwrap_err();

        assert_eq!(error.code, ErrorCode::PolicyDenied);
        assert_eq!(error.disposition, DeliverySemantics::not_delivered());
        assert_eq!(error.details.unwrap()["cause"], "window_not_focused");
        assert!(fake.activations().is_empty());
    }
}

#[test]
fn headless_focus_of_a_window_on_a_hidden_special_workspace_names_headed() {
    let mut fake = world(
        "special:scratch",
        snapshot(BYSTANDER, "1", None, false),
        None,
    );

    let error = focus_reported(&mut fake, &frame_window(), headless()).unwrap_err();

    assert_eq!(error.code, ErrorCode::PolicyDenied);
    assert!(error.suggestion.unwrap_or_default().contains("--headed"));
    let details = error.details.unwrap();
    assert_eq!(details["cause"], "hidden_special_workspace");
    assert_eq!(details["workspace"], "special:scratch");
    assert!(fake.activations().is_empty());
}

#[test]
fn headless_focus_refuses_when_the_compositor_and_the_frame_disagree() {
    let mut fake = world("1", snapshot(TARGET, "1", None, false), None);

    let error = focus_reported(&mut fake, &frame_window(), headless()).unwrap_err();

    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    let details = error.details.unwrap();
    assert_eq!(details["cause"], "frame_not_active");
    assert_eq!(details["activation_sent"], false);
    assert_eq!(details["compositor_active"], true);
    assert_eq!(details["frame_active"], false);
    assert!(fake.activations().is_empty());
}

#[test]
fn headed_focus_activates_the_bound_toplevel_and_reports_the_workspace_switch() {
    let mut fake = world(
        "2",
        snapshot(BYSTANDER, "1", None, false),
        Some(snapshot(TARGET, "2", None, true)),
    );

    let outcome = focus_reported(&mut fake, &frame_window(), headed()).expect("focused");

    assert_eq!(fake.activations(), vec!["activate 0x10"]);
    assert!(outcome.activation.activated);
    assert!(outcome.window.state.is_focused);
    assert_eq!(
        outcome.activation.side_effects,
        vec![FocusSideEffect::WorkspaceSwitched {
            monitor: "Virtual-1".into(),
            from: "1".into(),
            to: "2".into(),
        }]
    );
}

#[test]
fn headed_focus_reveals_a_hidden_special_workspace_and_says_so() {
    let mut fake = world(
        "special:scratch",
        snapshot(BYSTANDER, "1", None, false),
        Some(snapshot(TARGET, "1", Some("special:scratch"), true)),
    );

    let outcome = focus_reported(&mut fake, &frame_window(), headed()).expect("focused");

    assert_eq!(
        outcome.activation.side_effects,
        vec![FocusSideEffect::SpecialWorkspaceRevealed {
            monitor: "Virtual-1".into(),
            workspace: "special:scratch".into(),
        }]
    );
}

#[test]
fn headed_focus_waits_for_late_proof_within_its_budget() {
    let mut fake = world(
        "1",
        snapshot(BYSTANDER, "1", None, false),
        Some(snapshot(TARGET, "1", None, true)),
    );
    fake.lag = 3;

    let outcome = focus_reported(&mut fake, &frame_window(), headed()).expect("focused");

    assert!(outcome.activation.activated);
    assert_eq!(fake.activations().len(), 1);
}

#[test]
fn headed_focus_refuses_proof_that_never_arrives_and_reports_the_activation() {
    let mut fake = world(
        "2",
        snapshot(BYSTANDER, "1", None, false),
        Some(snapshot(BYSTANDER, "2", None, false)),
    );

    let error = focus_reported(&mut fake, &frame_window(), headed()).unwrap_err();

    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
    let details = error.details.unwrap();
    assert_eq!(details["cause"], "activation_refused");
    assert_eq!(details["activation_sent"], true);
    assert_eq!(
        details["side_effects"],
        serde_json::json!([{"kind": "workspace_switched", "monitor": "Virtual-1", "from": "1", "to": "2"}])
    );
    assert_eq!(fake.activations().len(), 1);
}

#[test]
fn headed_focus_refuses_when_the_frame_never_reports_active() {
    let mut fake = world(
        "1",
        snapshot(BYSTANDER, "1", None, false),
        Some(snapshot(TARGET, "1", None, false)),
    );

    let error = focus_reported(&mut fake, &frame_window(), headed()).unwrap_err();

    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
    let details = error.details.unwrap();
    assert_eq!(details["cause"], "frame_not_active");
    assert_eq!(details["compositor_active"], true);
    assert_eq!(details["toplevel_activated"], true);
}

#[test]
fn headed_focus_of_a_compositor_window_needs_no_frame() {
    let mut fake = world(
        "1",
        snapshot(BYSTANDER, "1", None, false),
        Some(snapshot(TARGET, "1", None, false)),
    );

    let outcome = focus_reported(&mut fake, &compositor_window("a"), headed()).expect("focused");

    assert_eq!(outcome.activation.proof, FocusProof::CompositorOnly);
    assert!(outcome.activation.activated);
    assert!(!fake.log.contains(&"frame".to_string()));
}

#[test]
fn an_unbindable_window_is_refused_before_any_activation() {
    let mut twins = world("1", snapshot(BYSTANDER, "1", None, false), None);
    twins.clients[2].title = "Notes".into();
    let mut restarted = world("1", snapshot(BYSTANDER, "1", None, false), None);
    restarted.live = Some("boot:101".into());
    let mut other = world("1", snapshot(BYSTANDER, "1", None, false), None);

    let twin_error = focus_reported(&mut twins, &frame_window(), headed()).unwrap_err();
    let stale_error = focus_reported(&mut restarted, &frame_window(), headed()).unwrap_err();
    let mut foreign = compositor_window("a");
    foreign.id = "w-hypr-50-61-a".into();
    let foreign_error = focus_reported(&mut other, &foreign, headed()).unwrap_err();
    let mut malformed = compositor_window("a");
    malformed.id = "w-hypr-x".into();
    let malformed_error = focus_reported(&mut other, &malformed, headed()).unwrap_err();

    assert_eq!(twin_error.details.unwrap()["cause"], "ambiguous_window");
    assert_eq!(stale_error.code, ErrorCode::StaleRef);
    assert_eq!(foreign_error.code, ErrorCode::WindowNotFound);
    assert_eq!(malformed_error.code, ErrorCode::WindowNotFound);
    for fake in [&twins, &restarted, &other] {
        assert!(fake.activations().is_empty());
    }
}

#[test]
fn an_open_special_workspace_over_the_targets_monitor_refuses_before_activation() {
    for policy in [headless(), headed()] {
        let mut fake = world(
            "3",
            snapshot(BYSTANDER, "1", Some("special:scratch"), false),
            Some(snapshot(TARGET, "3", None, true)),
        );

        let error = focus_reported(&mut fake, &frame_window(), policy).unwrap_err();

        assert_eq!(error.code, ErrorCode::ActionFailed);
        assert_eq!(error.disposition, DeliverySemantics::not_delivered());
        let details = error.details.unwrap();
        assert_eq!(details["cause"], "special_workspace_open");
        assert_eq!(details["workspace"], "special:scratch");
        assert_eq!(details["monitor"], "Virtual-1");
        assert!(fake.activations().is_empty());
    }
}

#[test]
fn headed_focus_reports_the_cursor_the_activation_moved() {
    let mut before = snapshot(BYSTANDER, "1", None, false);
    before.desktop.cursor = Some(Point { x: 322.0, y: 413.0 });
    let mut after = snapshot(TARGET, "1", None, true);
    after.desktop.cursor = Some(Point { x: 957.0, y: 604.0 });
    let mut fake = world("1", before, Some(after));

    let outcome = focus_reported(&mut fake, &frame_window(), headed()).expect("focused");

    assert_eq!(
        outcome.activation.side_effects,
        vec![FocusSideEffect::CursorMoved {
            from: Point { x: 322.0, y: 413.0 },
            to: Point { x: 957.0, y: 604.0 },
        }]
    );
}
