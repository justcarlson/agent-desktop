use agent_desktop_core::{DeliverySemantics, ErrorCode, WindowOp};
use serde_json::json;

use super::*;
use crate::system::hyprland::window_op::client_state::ClientState;
use crate::system::hyprland::window_op::window_op_fixture::{
    HOLDER, LEGACY_REPLY, LUA_REPLY, TARGET, desktop, floating_at, frame, holder, state, window,
    world,
};

#[test]
fn a_move_is_sent_by_address_and_proven_by_the_read_back() {
    let mut fake = world(
        LUA_REPLY,
        vec![
            frame(floating_at((0, 0)), Some(HOLDER)),
            frame(floating_at((40, 60)), Some(HOLDER)),
        ],
    );

    let report = window_op_reported(&mut fake, &window(), &WindowOp::Move { x: 40.0, y: 60.0 })
        .expect("moved");

    assert_eq!(
        report.disposition(),
        DeliverySemantics::delivered_verified()
    );
    assert_eq!(report.result.action, "move-window");
    let details = report.result.details.expect("details");
    assert_eq!(details["verification_scope"], "window_position");
    assert_eq!(details["observed"]["x"], 40);
    assert_eq!(details.get("side_effects"), None);
    assert_eq!(
        fake.dispatches(),
        vec!["dispatch hl.dsp.window.move({x=40,y=60,window=\"address:0x10\"})"]
    );
    assert_eq!(fake.log[0], "eval return 1");
}

#[test]
fn a_resize_is_proven_only_after_the_corner_is_back() {
    let mut fake = world(
        LUA_REPLY,
        vec![
            frame(floating_at((40, 60)), Some(HOLDER)),
            frame(state(TARGET, (105, 238), (500, 400)), Some(HOLDER)),
            frame(state(TARGET, (40, 60), (500, 400)), Some(HOLDER)),
        ],
    );

    let report = window_op_reported(
        &mut fake,
        &window(),
        &WindowOp::Resize {
            width: 500.0,
            height: 400.0,
        },
    )
    .expect("resized");

    assert_eq!(
        report.disposition(),
        DeliverySemantics::delivered_verified()
    );
    assert_eq!(
        report.result.details.expect("details")["verification_scope"],
        "window_size"
    );
    assert_eq!(
        fake.dispatches(),
        vec![
            "dispatch hl.dsp.window.resize({x=500,y=400,window=\"address:0x10\"})",
            "dispatch hl.dsp.window.move({x=40,y=60,window=\"address:0x10\"})",
        ]
    );
}

#[test]
fn a_clamped_resize_still_puts_the_corner_back() {
    let mut fake = world(
        LUA_REPLY,
        vec![
            frame(floating_at((40, 60)), Some(HOLDER)),
            frame(state(TARGET, (351, 429), (7, 17)), Some(HOLDER)),
            frame(state(TARGET, (40, 60), (7, 17)), Some(HOLDER)),
        ],
    );

    let error = window_op_reported(
        &mut fake,
        &window(),
        &WindowOp::Resize {
            width: 5.0,
            height: 5.0,
        },
    )
    .expect_err("clamped");

    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
    let details = error.details.expect("details");
    assert_eq!(details["cause"], "readback_mismatch");
    assert_eq!(details["observed"]["x"], 40);
    assert_eq!(details["observed"]["y"], 60);
    assert_eq!(
        fake.dispatches(),
        vec![
            "dispatch hl.dsp.window.resize({x=5,y=5,window=\"address:0x10\"})",
            "dispatch hl.dsp.window.move({x=40,y=60,window=\"address:0x10\"})",
        ]
    );
}

#[test]
fn minimizing_the_focused_window_reports_where_focus_went() {
    let parked = ClientState {
        workspace: "special:agent-desktop-minimized-1".into(),
        ..floating_at((40, 60))
    };
    let mut fake = world(
        LUA_REPLY,
        vec![
            frame(floating_at((40, 60)), Some(TARGET)),
            frame(parked, Some(HOLDER)),
        ],
    );

    let report = window_op_reported(&mut fake, &window(), &WindowOp::Minimize).expect("minimized");

    assert_eq!(
        report.disposition(),
        DeliverySemantics::delivered_verified()
    );
    let details = report.result.details.expect("details");
    assert_eq!(details["verification_scope"], "window_minimized_state");
    assert_eq!(
        details["side_effects"],
        json!([{"kind": "active_window_changed", "from": "0x10", "to": "0x30"}])
    );
}

#[test]
fn restore_is_proven_by_the_restored_state_scope() {
    let parked = ClientState {
        workspace: "special:agent-desktop-minimized-1".into(),
        fullscreen: 1,
        ..floating_at((40, 60))
    };
    let back = ClientState {
        fullscreen: 1,
        ..floating_at((40, 60))
    };
    let mut fake = world(
        LUA_REPLY,
        vec![
            frame(parked, Some(HOLDER)),
            frame(back, Some(HOLDER)),
            frame(floating_at((40, 60)), Some(HOLDER)),
        ],
    );

    let report = window_op_reported(&mut fake, &window(), &WindowOp::Restore).expect("restored");

    assert_eq!(
        report.disposition(),
        DeliverySemantics::delivered_verified()
    );
    assert_eq!(
        report.result.details.expect("details")["verification_scope"],
        "window_restored_state"
    );
    assert_eq!(fake.dispatches().len(), 2);
}

const MAXIMIZE: &str = "dispatch hl.dsp.window.fullscreen({mode=\"maximized\",action=\"set\",window=\"address:0x10\"})";
const UNSET: &str = "dispatch hl.dsp.window.fullscreen_state({internal=0,client=0,action=\"set\",window=\"address:0x10\"})";

#[test]
fn a_maximize_sends_only_the_maximize() {
    let maximized = ClientState {
        fullscreen: 1,
        ..floating_at((40, 60))
    };
    let mut fake = world(
        LUA_REPLY,
        vec![
            frame(floating_at((40, 60)), Some(TARGET)),
            frame(maximized, Some(TARGET)),
        ],
    );

    window_op_reported(&mut fake, &window(), &WindowOp::Maximize).expect("maximized");

    assert_eq!(fake.dispatches(), vec![MAXIMIZE]);
}

#[test]
fn a_restore_after_the_output_moved_leaves_the_window_where_hyprland_put_it() {
    let maximized = ClientState {
        fullscreen: 1,
        size: (1024, 640),
        ..floating_at((-1024, -640))
    };
    let mut fake = world(
        LUA_REPLY,
        vec![
            frame(maximized, Some(TARGET)),
            frame(floating_at((40, 60)), Some(TARGET)),
        ],
    );

    let report = window_op_reported(&mut fake, &window(), &WindowOp::Restore).expect("restored");

    assert_eq!(
        report.disposition(),
        DeliverySemantics::delivered_verified()
    );
    assert_eq!(fake.dispatches(), vec![UNSET]);
    assert_eq!(report.result.details.expect("details")["observed"]["x"], 40);
}

#[test]
fn an_operation_already_satisfied_is_not_delivered() {
    let mut fake = world(LUA_REPLY, vec![frame(floating_at((40, 60)), Some(HOLDER))]);

    let report = window_op_reported(&mut fake, &window(), &WindowOp::Move { x: 40.0, y: 60.0 })
        .expect("satisfied");

    assert_eq!(report.disposition(), DeliverySemantics::not_delivered());
    assert_eq!(report.result.action, "move-window");
    assert!(fake.dispatches().is_empty());
}

#[test]
fn a_legacy_config_gets_the_legacy_dispatchers() {
    let mut fake = world(
        LEGACY_REPLY,
        vec![
            frame(floating_at((0, 0)), Some(HOLDER)),
            frame(floating_at((40, 60)), Some(HOLDER)),
        ],
    );

    window_op_reported(&mut fake, &window(), &WindowOp::Move { x: 40.0, y: 60.0 }).expect("moved");

    assert_eq!(
        fake.dispatches(),
        vec!["dispatch movewindowpixel exact 40 60,address:0x10"]
    );
}

#[test]
fn an_unknown_config_dialect_is_refused_before_anything_is_sent() {
    let mut fake = world("unknown request", vec![frame(floating_at((0, 0)), None)]);

    let error = window_op_reported(&mut fake, &window(), &WindowOp::Move { x: 40.0, y: 60.0 })
        .expect_err("refused");

    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert_eq!(
        error.details.expect("details")["cause"],
        "config_dialect_unknown"
    );
    assert!(fake.dispatches().is_empty());
}

#[test]
fn a_read_back_that_never_holds_is_delivered_unverified() {
    let mut fake = world(LUA_REPLY, vec![frame(floating_at((0, 0)), Some(HOLDER))]);

    let error = window_op_reported(&mut fake, &window(), &WindowOp::Move { x: 40.0, y: 60.0 })
        .expect_err("unverified");

    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
    let details = error.details.expect("details");
    assert_eq!(details["cause"], "readback_mismatch");
    assert_eq!(details["requested"], json!([{"x": 40, "y": 60}]));
    assert_eq!(details["observed"]["x"], 0);
    assert_eq!(fake.log.iter().filter(|entry| *entry == "wait").count(), 3);
}

#[test]
fn a_compositor_rejection_is_not_delivered() {
    let mut fake = world(LUA_REPLY, vec![frame(floating_at((0, 0)), Some(HOLDER))]);
    fake.reply = "warning: =[C]:-1: Window is fullscreen".into();

    let error = window_op_reported(&mut fake, &window(), &WindowOp::Move { x: 40.0, y: 60.0 })
        .expect_err("rejected");

    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    let details = error.details.expect("details");
    assert_eq!(details["cause"], "compositor_rejected");
    assert_eq!(details["reply"], "warning: =[C]:-1: Window is fullscreen");
}

#[test]
fn a_window_missing_from_the_client_list_is_not_found() {
    let mut fake = world(LUA_REPLY, vec![(vec![holder()], desktop(Some(HOLDER)))]);

    let error = window_op_reported(&mut fake, &window(), &WindowOp::Minimize).expect_err("gone");

    assert_eq!(error.code, ErrorCode::WindowNotFound);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert!(fake.dispatches().is_empty());
}

#[test]
fn an_invalid_size_is_refused_before_the_compositor_is_read() {
    let mut fake = world(LUA_REPLY, vec![frame(floating_at((0, 0)), None)]);

    let error = window_op_reported(
        &mut fake,
        &window(),
        &WindowOp::Resize {
            width: 0.0,
            height: 400.0,
        },
    )
    .expect_err("invalid");

    assert_eq!(error.code, ErrorCode::InvalidArgs);
    assert!(fake.log.is_empty());
}
