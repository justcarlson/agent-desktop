use super::*;
use crate::system::hyprland::clients::monitor_workspaces::MonitorWorkspaces;
use agent_desktop_core::Point;

fn monitor(name: &str, active: &str, special: Option<&str>) -> MonitorWorkspaces {
    MonitorWorkspaces {
        monitor: name.into(),
        id: Some(0),
        active: active.into(),
        special: special.map(str::to_owned),
    }
}

fn state(monitors: Vec<MonitorWorkspaces>) -> DesktopState {
    DesktopState {
        active: None,
        monitors,
        cursor: None,
    }
}

#[test]
fn an_unchanged_desktop_has_no_side_effects() {
    let before = state(vec![monitor("Virtual-1", "1", None)]);

    assert!(side_effects(&before, &before.clone()).is_empty());
}

#[test]
fn a_changed_active_workspace_is_a_switch_on_its_monitor() {
    let before = state(vec![
        monitor("HDMI-A-1", "4", None),
        monitor("Virtual-1", "1", None),
    ]);
    let after = state(vec![
        monitor("HDMI-A-1", "4", None),
        monitor("Virtual-1", "2", None),
    ]);

    assert_eq!(
        side_effects(&before, &after),
        vec![FocusSideEffect::WorkspaceSwitched {
            monitor: "Virtual-1".into(),
            from: "1".into(),
            to: "2".into(),
        }]
    );
}

#[test]
fn an_opened_special_workspace_is_revealed_and_a_gone_one_is_closed() {
    let before = state(vec![monitor("Virtual-1", "1", Some("special:a"))]);
    let after = state(vec![monitor("Virtual-1", "1", Some("special:b"))]);

    assert_eq!(
        side_effects(&before, &after),
        vec![
            FocusSideEffect::SpecialWorkspaceClosed {
                monitor: "Virtual-1".into(),
                workspace: "special:a".into(),
            },
            FocusSideEffect::SpecialWorkspaceRevealed {
                monitor: "Virtual-1".into(),
                workspace: "special:b".into(),
            },
        ]
    );
}

#[test]
fn a_special_workspace_that_stays_open_is_no_side_effect() {
    let before = state(vec![monitor("Virtual-1", "1", None)]);
    let opened = state(vec![monitor("Virtual-1", "1", Some("special:hid"))]);

    assert_eq!(
        side_effects(&before, &opened),
        vec![FocusSideEffect::SpecialWorkspaceRevealed {
            monitor: "Virtual-1".into(),
            workspace: "special:hid".into(),
        }]
    );
    assert!(side_effects(&opened, &opened.clone()).is_empty());
    assert_eq!(
        side_effects(&opened, &before),
        vec![FocusSideEffect::SpecialWorkspaceClosed {
            monitor: "Virtual-1".into(),
            workspace: "special:hid".into(),
        }]
    );
}

#[test]
fn a_monitor_present_on_only_one_side_is_not_diffed() {
    let before = state(vec![monitor("Virtual-1", "1", None)]);
    let after = state(vec![
        monitor("Virtual-1", "1", None),
        monitor("HDMI-A-1", "9", Some("special:x")),
    ]);

    assert!(side_effects(&before, &after).is_empty());
}

#[test]
fn a_moved_cursor_is_reported_and_an_unread_one_is_not() {
    let at = |x: f64, y: f64| Some(Point { x, y });
    let mut before = state(vec![monitor("Virtual-1", "1", None)]);
    before.cursor = at(322.0, 413.0);
    let mut after = before.clone();
    after.cursor = at(957.0, 604.0);

    assert_eq!(
        side_effects(&before, &after),
        vec![FocusSideEffect::CursorMoved {
            from: Point { x: 322.0, y: 413.0 },
            to: Point { x: 957.0, y: 604.0 },
        }]
    );
    assert!(side_effects(&before, &before.clone()).is_empty());
    let mut unread = after.clone();
    unread.cursor = None;
    assert!(side_effects(&before, &unread).is_empty());
}
