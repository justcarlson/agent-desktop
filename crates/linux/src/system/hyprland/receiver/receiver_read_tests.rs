use agent_desktop_core::{ErrorCode, HitTestResult};

use super::*;
use crate::system::hyprland::receiver::fake_stack_ipc::{CannedReply, FakeStackIpc};
use crate::system::hyprland::receiver::receiver_hit::read_hit;

const TARGET: u64 = 0xa1;
const POINT: Point = Point { x: 162.0, y: 238.0 };

const CLIENT_ON_WORKSPACE_2: &str = r#"[
 {"address":"0xa1","mapped":true,"hidden":false,"at":[12,38],"size":[1256,750],
  "workspace":{"id":2,"name":"2"},"floating":false,"monitor":0,"pid":3704,
  "pinned":false,"fullscreen":0}]"#;

const MONITOR_ON_WORKSPACE_1: &str = r#"[
 {"name":"Virtual-1","id":0,"activeWorkspace":{"id":1,"name":"1"},
  "specialWorkspace":{"id":0,"name":""}}]"#;

fn deadline() -> Deadline {
    Deadline::after(300).expect("deadline")
}

fn gone_binding() -> HyprlandBinding {
    HyprlandBinding {
        signature: "test".into(),
        ipc_socket: std::path::PathBuf::from("/nonexistent/hypr/x/.socket.sock"),
        wayland_socket: std::path::PathBuf::from("/nonexistent/wayland-1"),
        pid: std::process::id(),
        start_time: 0,
    }
}

fn cause(error: &AdapterError) -> &str {
    error
        .details
        .as_ref()
        .and_then(|details| details["cause"].as_str())
        .unwrap_or_default()
}

#[test]
fn a_silent_stack_read_is_a_timeout_for_the_hit_test() {
    let fake = FakeStackIpc::silent("hit-timeout");
    let error = read_hit(&fake.binding, TARGET, &POINT, deadline())
        .expect_err("a read that ran out of time is not an unknown hit");
    assert_eq!(error.code, ErrorCode::Timeout);
}

#[test]
fn a_stack_read_from_a_gone_instance_is_an_unknown_hit() {
    let hit = read_hit(&gone_binding(), TARGET, &POINT, deadline());
    assert_eq!(hit.expect("not an error"), HitTestResult::Unknown);
}

#[test]
fn a_failed_stack_read_refuses_the_receiver() {
    let error = prove_receiver(&gone_binding(), TARGET, &POINT, deadline())
        .expect_err("no stack, no proof");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(cause(&error), "evidence_missing");
}

#[test]
fn a_target_on_a_hidden_workspace_refuses_the_receiver() {
    let fake = FakeStackIpc::answering(
        "unshown",
        vec![
            CannedReply {
                command: "j/clients",
                body: CLIENT_ON_WORKSPACE_2,
            },
            CannedReply {
                command: "j/layers",
                body: "{}",
            },
            CannedReply {
                command: "j/activewindow",
                body: "{}",
            },
            CannedReply {
                command: "j/monitors",
                body: MONITOR_ON_WORKSPACE_1,
            },
        ],
    );
    let error = prove_receiver(&fake.binding, TARGET, &POINT, deadline())
        .expect_err("a target on a workspace no output shows is refused");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(cause(&error), "target_not_shown");
}
