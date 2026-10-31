use super::*;
use crate::system::hyprland::clients::client_workspace::ClientWorkspace;

const MONITORS: &str = r#"[
  {"id":1,"name":"HDMI-A-1","focused":false,"activeWorkspace":{"id":4,"name":"4"},
   "specialWorkspace":{"id":0,"name":""}},
  {"id":0,"name":"Virtual-1","focused":true,"activeWorkspace":{"id":1,"name":"1"},
   "specialWorkspace":{"id":-98,"name":"special:hid"}}
]"#;

#[test]
fn every_monitor_reports_its_active_and_special_workspace_by_name() {
    let monitors = parse_monitors(MONITORS.as_bytes()).expect("monitors");

    assert_eq!(
        monitors,
        vec![
            MonitorWorkspaces {
                monitor: "HDMI-A-1".into(),
                id: Some(1),
                active: "4".into(),
                special: None,
            },
            MonitorWorkspaces {
                monitor: "Virtual-1".into(),
                id: Some(0),
                active: "1".into(),
                special: Some("special:hid".into()),
            },
        ]
    );
}

#[test]
fn a_monitor_without_an_active_workspace_makes_the_reply_unusable() {
    let broken = r#"[{"id":0,"name":"Virtual-1","specialWorkspace":{"id":0,"name":""}}]"#;

    assert!(parse_monitors(broken.as_bytes()).is_none());
}

#[test]
fn a_shown_special_workspace_counts_as_visible_and_a_hidden_one_does_not() {
    let monitors = parse_monitors(MONITORS.as_bytes()).expect("monitors");
    let state = DesktopState {
        active: None,
        monitors,
        cursor: None,
    };

    assert!(state.shows("special:hid"));
    assert!(state.shows("4"));
    assert!(!state.shows("special:other"));
    assert!(!state.shows("2"));
}

#[test]
fn the_cursor_position_is_read_from_its_reply_and_anything_else_is_none() {
    assert_eq!(
        parse_cursor(br#"{"x":957,"y":604}"#),
        Some(Point { x: 957.0, y: 604.0 })
    );
    assert_eq!(parse_cursor(br#"{"x":957}"#), None);
    assert_eq!(parse_cursor(b"640, 400"), None);
}

#[test]
fn an_open_special_workspace_covers_a_regular_workspace_on_its_monitor() {
    let monitors = parse_monitors(MONITORS.as_bytes()).expect("monitors");
    let state = DesktopState {
        active: None,
        monitors,
        cursor: None,
    };
    let on = |name: &str, monitor: i64| ClientWorkspace {
        name: name.into(),
        monitor: Some(monitor),
    };

    let covering = state.covering_special(&on("3", 0)).expect("covered");
    assert_eq!(
        (covering.monitor.as_str(), covering.special.as_deref()),
        ("Virtual-1", Some("special:hid"))
    );
    assert!(state.covering_special(&on("special:hid", 0)).is_none());
    assert!(state.covering_special(&on("special:other", 0)).is_none());
    assert!(state.covering_special(&on("4", 1)).is_none());
    let unplaced = ClientWorkspace {
        name: "3".into(),
        monitor: None,
    };
    assert!(state.covering_special(&unplaced).is_none());
}
