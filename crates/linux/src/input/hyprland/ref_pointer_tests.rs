use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode, Point};

use super::*;
use crate::actions::physical_target::tests::fake::{FakeWorld, element};
use crate::system::hyprland::receiver::fake_stack_ipc::{CannedReply, FakeStackIpc};

pub(crate) const TARGET: u64 = 0xa1;

const XWAYLAND_CLIENT: &str = r#"[
 {"address":"0xa1","mapped":true,"hidden":false,"at":[0,0],"size":[640,480],
  "workspace":{"id":1,"name":"1"},"floating":false,"monitor":0,"pid":3704,
  "pinned":false,"fullscreen":0,"xwayland":true}]"#;

const MONITOR_ON_WORKSPACE_1: &str = r#"[
 {"name":"Virtual-1","id":0,"activeWorkspace":{"id":1,"name":"1"},
  "specialWorkspace":{"id":0,"name":""}}]"#;

/// A bound instance whose stack proves that every point inside the
/// XWayland toplevel `0xa1` reaches it, for as many reads as a gesture asks.
pub(crate) fn proven_xwayland_stack(name: &str) -> FakeStackIpc {
    let replies = (0..12)
        .flat_map(|_| {
            [
                CannedReply {
                    command: "j/clients",
                    body: XWAYLAND_CLIENT,
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
            ]
        })
        .collect();
    FakeStackIpc::answering(name, replies)
}

pub(crate) fn placed() -> PlacedTarget {
    PlacedTarget {
        address: TARGET,
        at: Point { x: 100.0, y: 100.0 },
    }
}

fn deadline() -> Deadline {
    Deadline::detached_after(3_000).expect("deadline")
}

pub(crate) fn assert_unproven(error: &AdapterError) {
    assert_eq!(error.code, ErrorCode::ActionFailed, "{error:?}");
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    let details = error.details.as_ref().expect("refusal evidence");
    assert_eq!(details["kind"], "receiver_evidence");
    assert_eq!(details["cause"], "receiver_unproven");
    assert_eq!(details["physical_delivery_started"], false);
    let suggestion = error.suggestion.as_deref().unwrap_or_default();
    assert!(suggestion.contains("without --headed"), "{suggestion}");
    assert!(suggestion.contains("--xy"), "{suggestion}");
}

#[test]
fn a_headed_click_on_an_xwayland_target_with_a_stack_proof_sends_nothing() {
    let mut reached = Vec::new();
    let clicks = [
        ("click", ClickSpec::left(1, None)),
        ("double", ClickSpec::left(2, None)),
        ("triple", ClickSpec::left(3, None)),
        ("right", ClickSpec::right(None)),
    ];
    for (name, spec) in clicks {
        let fake = proven_xwayland_stack(&format!("xwayland-{name}"));
        let world = FakeWorld::wayland();
        let element = element();
        let route = RefPointer {
            binding: &fake.binding,
            element: &element,
            deadline: deadline(),
        };
        let result = route.click_placed(&world, &spec, &placed());
        if !world.sent_nothing() || result.is_ok() {
            reached.push(format!("{name}: {:?} {:?}", world.entries(), result.err()));
            continue;
        }
        assert_unproven(&result.expect_err(name));
    }
    assert!(reached.is_empty(), "events reached the app: {reached:#?}");
}

#[test]
fn a_headed_scroll_on_an_xwayland_target_with_a_stack_proof_sends_nothing() {
    let fake = proven_xwayland_stack("xwayland-scroll");
    let world = FakeWorld::wayland();
    let element = element();
    let route = RefPointer {
        binding: &fake.binding,
        element: &element,
        deadline: deadline(),
    };
    let result = route.scroll_placed(&world, (0.0, -3.0), &placed());
    assert!(world.sent_nothing(), "{:?}", world.entries());
    assert_unproven(&result.expect_err("scroll"));
}
