use std::process::Command;

use serde_json::Value;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;

use crate::system::x11::xwd_oracle::xwd_pixel;

fn cli(args: &[&str]) -> Value {
    let binary = std::env::var_os("AGENT_DESKTOP_TEST_BINARY").expect("candidate CLI");
    let output = Command::new(binary)
        .args(args)
        .env_remove("AGENT_DESKTOP_SESSION")
        .env_remove("AGENT_DESKTOP_AGENT_ID")
        .output()
        .expect("CLI process exits");
    let answer: Value = serde_json::from_slice(&output.stdout).expect("JSON envelope");
    assert!(output.status.success(), "{answer}");
    assert_eq!(answer["ok"], true, "{answer}");
    answer["data"].clone()
}

fn owned(connection: &RustConnection, root: Window, session: &str) -> Vec<Window> {
    let owner = connection
        .intern_atom(false, b"_AGENT_DESKTOP_CURSOR_OWNER")
        .unwrap()
        .reply()
        .unwrap()
        .atom;
    connection
        .query_tree(root)
        .unwrap()
        .reply()
        .unwrap()
        .children
        .into_iter()
        .filter(|window| {
            connection
                .get_property(false, *window, owner, AtomEnum::STRING, 0, 1024)
                .unwrap()
                .reply()
                .ok()
                .and_then(|property| {
                    serde_json::from_slice::<(String, Option<String>)>(&property.value).ok()
                })
                .is_some_and(|(id, _)| id == session)
        })
        .collect()
}

fn one_window(connection: &RustConnection, root: Window, session: &str, dir: &str) -> Window {
    let windows = owned(connection, root, session);
    assert_eq!(windows.len(), 1, "rendered:true must have a native window");
    let window = windows[0];
    let attrs = connection
        .get_window_attributes(window)
        .unwrap()
        .reply()
        .unwrap();
    assert!(attrs.override_redirect);
    assert_eq!(attrs.map_state, MapState::VIEWABLE);
    assert_eq!(
        connection
            .get_geometry(window)
            .unwrap()
            .reply()
            .unwrap()
            .depth,
        32
    );
    assert_eq!(xwd_pixel(window, 3, 10, dir), [255, 255, 255]);
    window
}

#[test]
#[ignore = "private X11 desktop and AGENT_DESKTOP_TEST_BINARY pointing to the candidate CLI"]
fn live_x11_cursor_cli_reports_rendered_and_paints_agent_positions() {
    let dir = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("private desktop");
    let (oracle, index) = x11rb::connect(None).expect("native window oracle");
    let root = oracle.setup().roots[index].root;
    let started = cli(&["session", "start", "--cursor", "--no-trace"]);
    assert_eq!(started["rendered"], true, "{started}");
    let session = started["session_id"].as_str().expect("session id");
    one_window(&oracle, root, session, &dir);
    cli(&["--session", session, "cursor-overlay", "disable"]);
    assert!(owned(&oracle, root, session).is_empty());
    let enabled = cli(&["--session", session, "cursor-overlay", "enable"]);
    assert_eq!(enabled["rendered"], true, "{enabled}");
    let window = one_window(&oracle, root, session, &dir);
    for (xy, expected) in [("150,150", (150, 150)), ("500,400", (500, 400))] {
        cli(&["--session", session, "--headed", "mouse-move", "--xy", xy]);
        assert_eq!(one_window(&oracle, root, session, &dir), window);
        let geometry = oracle.get_geometry(window).unwrap().reply().unwrap();
        assert_eq!((geometry.x, geometry.y), expected);
    }
    cli(&["--session", session, "cursor-overlay", "disable"]);
    assert!(owned(&oracle, root, session).is_empty());
    let enabled = cli(&["--session", session, "cursor-overlay", "enable"]);
    assert_eq!(enabled["rendered"], true, "{enabled}");
    one_window(&oracle, root, session, &dir);
    cli(&["session", "end", session]);
    assert!(owned(&oracle, root, session).is_empty());
}

#[test]
#[ignore = "private headless Wayland and AGENT_DESKTOP_TEST_BINARY pointing to the candidate CLI"]
fn live_wayland_cursor_cli_reports_unrendered_without_x11_fallback() {
    std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("private desktop");
    assert!(!std::env::var("WAYLAND_DISPLAY").unwrap().is_empty());
    let (oracle, index) = x11rb::connect(None).expect("private X11 fallback oracle");
    let root = oracle.setup().roots[index].root;
    let started = cli(&["session", "start", "--cursor", "--no-trace"]);
    assert_eq!(started["rendered"], false, "{started}");
    let session = started["session_id"].as_str().expect("session id");
    assert!(owned(&oracle, root, session).is_empty());
    let enabled = cli(&["--session", session, "cursor-overlay", "enable"]);
    assert_eq!(enabled["rendered"], false, "{enabled}");
    assert!(owned(&oracle, root, session).is_empty());
    cli(&["--session", session, "cursor-overlay", "disable"]);
    cli(&["session", "end", session]);
    assert!(owned(&oracle, root, session).is_empty());
}
