use agent_desktop_core::{DeliverySemantics, ErrorCode, WindowOp};

use super::*;
use crate::system::hyprland::window_op::client_marks::ClientMarks;

const MOVED: [OutputOrigin; 1] = [OutputOrigin {
    id: 0,
    at: (-1024, -640),
}];
const HOME: [OutputOrigin; 1] = [OutputOrigin { id: 0, at: (0, 0) }];
const RECORD: &str = "agent-desktop-restore:40,60,500,400";
const STALE: &str = "agent-desktop-restore:1,1,9,9";

fn window(at: (i64, i64), size: (i64, i64), tags: &[&str]) -> ClientState {
    ClientState {
        address: 0x10,
        at,
        size,
        floating: true,
        fullscreen: 0,
        workspace: "1".into(),
        marks: ClientMarks {
            monitor: Some(0),
            tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
        },
    }
}

fn commands(steps: &[WindowStep]) -> Vec<&str> {
    steps.iter().map(|step| step.command.as_str()).collect()
}

#[test]
fn a_maximize_removes_stale_records_then_records_the_offset_from_the_monitor() {
    let state = window((-984, -580), (500, 400), &[STALE, "work"]);

    let steps = recorded(&state, &MOVED).expect("recorded");

    assert_eq!(
        commands(&steps),
        vec![
            "dispatch hl.dsp.window.tag({tag=\"-agent-desktop-restore:1,1,9,9\",window=\"address:0x10\"})",
            "dispatch hl.dsp.window.tag({tag=\"+agent-desktop-restore:40,60,500,400\",window=\"address:0x10\"})",
        ]
    );
    assert_eq!(steps[1].goals, vec![StepGoal::Tagged(RECORD.into())]);
}

#[test]
fn a_tiled_window_gets_no_record() {
    let state = ClientState {
        floating: false,
        ..window((0, 0), (500, 400), &[STALE])
    };

    let steps = recorded(&state, &[]).expect("recorded");

    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].goals, vec![StepGoal::Untagged(STALE.into())]);
}

#[test]
fn a_floating_window_whose_monitor_has_no_origin_is_refused() {
    let unplaced = ClientState {
        marks: ClientMarks::default(),
        ..window((40, 60), (500, 400), &[])
    };

    for (state, origins) in [
        (window((40, 60), (500, 400), &[]), &[][..]),
        (unplaced, &HOME[..]),
    ] {
        let error = recorded(&state, origins).expect_err("refused");
        assert_eq!(error.disposition, DeliverySemantics::not_delivered());
        assert_eq!(
            error.details.expect("details")["cause"],
            "window_state_unreadable"
        );
    }
}

#[test]
fn a_restore_puts_the_window_at_its_offset_from_the_moved_monitor() {
    let held = ClientState {
        fullscreen: 1,
        ..window((-1024, -640), (1024, 640), &[RECORD])
    };
    let after = window((40, 60), (500, 400), &[RECORD]);

    let steps = placed(&held, &after, &MOVED).expect("placed");

    assert_eq!(
        commands(&steps),
        vec![
            "dispatch hl.dsp.window.move({x=-984,y=-580,window=\"address:0x10\"})",
            "dispatch hl.dsp.window.tag({tag=\"-agent-desktop-restore:40,60,500,400\",window=\"address:0x10\"})",
        ]
    );
    assert_eq!(
        steps[0].goals,
        vec![StepGoal::At(-984, -580), StepGoal::Size(500, 400)]
    );
}

#[test]
fn a_restore_at_another_size_resizes_before_it_moves() {
    let after = window((40, 60), (630, 755), &[RECORD]);

    let steps = placed(&after, &after, &MOVED).expect("placed");

    assert_eq!(
        steps[0].goals,
        vec![StepGoal::Resized {
            wanted: (500, 400),
            from: (630, 755)
        }]
    );
    assert_eq!(steps[1].goals[0], StepGoal::At(-984, -580));
    assert_eq!(steps.len(), 3);
}

#[test]
fn a_window_already_in_place_only_loses_its_record() {
    let after = window((40, 60), (500, 400), &[RECORD]);

    let steps = placed(&after, &after, &HOME).expect("placed");

    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].goals, vec![StepGoal::Untagged(RECORD.into())]);
}

#[test]
fn two_records_or_a_tiled_window_are_left_where_hyprland_put_them() {
    let twice = window((40, 60), (630, 755), &[RECORD, STALE]);
    let tiled = ClientState {
        floating: false,
        ..window((40, 60), (630, 755), &[RECORD])
    };

    assert_eq!(placed(&twice, &twice, &MOVED).expect("placed").len(), 2);
    assert_eq!(placed(&tiled, &tiled, &MOVED).expect("placed").len(), 1);
}

#[test]
fn a_restore_whose_monitor_is_gone_is_delivered_unverified() {
    let after = window((40, 60), (500, 400), &[RECORD]);

    let error = placed(&after, &after, &[]).expect_err("unplaced");

    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
    let details = error.details.expect("details");
    assert_eq!(details["cause"], "window_state_unreadable");
    assert_eq!(details["physical_delivery_started"], true);
}

#[test]
fn only_a_floating_maximize_or_a_recorded_restore_reads_the_origins() {
    let floating = window((0, 0), (500, 400), &[]);
    let maximized = ClientState {
        fullscreen: 1,
        ..window((0, 0), (500, 400), &[RECORD])
    };
    let unrecorded = ClientState {
        fullscreen: 1,
        ..floating.clone()
    };

    let tiled = ClientState {
        floating: false,
        ..floating.clone()
    };

    assert!(needs_origins(&WindowOp::Maximize, &floating));
    assert!(!needs_origins(&WindowOp::Maximize, &tiled));
    assert!(needs_origins(&WindowOp::Restore, &maximized));
    assert!(!needs_origins(&WindowOp::Minimize, &maximized));
    assert!(!needs_origins(&WindowOp::Restore, &unrecorded));
    assert!(!needs_origins(&WindowOp::Restore, &floating));
}
