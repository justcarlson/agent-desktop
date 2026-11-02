use agent_desktop_core::{DeliverySemantics, ErrorCode, WindowOp};
use serde_json::json;

use super::*;
use crate::system::hyprland::window_op::client_state::ClientState;
use crate::system::hyprland::window_op::output_origin::OutputOrigin;
use crate::system::hyprland::window_op::window_op_fixture::{
    HOLDER, LUA_REPLY, TARGET, UNSET, UNTAG, floating_at, frame, tagged, window, world,
};

const MOVED: OutputOrigin = OutputOrigin {
    id: 0,
    at: (-1024, -640),
};
const PLACE: &str = "dispatch hl.dsp.window.move({x=-984,y=-580,window=\"address:0x10\"})";

fn maximized() -> ClientState {
    ClientState {
        fullscreen: 1,
        ..tagged(floating_at((-1024, -640)))
    }
}

#[test]
fn a_recorded_restore_whose_monitor_has_no_origin_sends_nothing() {
    let unplaced = {
        let mut state = maximized();
        state.marks.monitor = None;
        state
    };
    for (state, origins) in [
        (maximized(), vec![OutputOrigin { id: 1, at: (0, 0) }]),
        (unplaced, vec![MOVED]),
    ] {
        let mut fake = world(
            LUA_REPLY,
            vec![
                frame(state, Some(TARGET)),
                frame(tagged(floating_at((40, 60))), Some(TARGET)),
            ],
        );
        fake.origins = Some(origins);

        let error =
            window_op_reported(&mut fake, &window(), &WindowOp::Restore).expect_err("refused");

        assert_eq!(error.code, ErrorCode::ActionNotSupported);
        assert_eq!(error.disposition, DeliverySemantics::not_delivered());
        let details = error.details.expect("details");
        assert_eq!(details["cause"], "window_state_unreadable");
        assert_eq!(details["physical_delivery_started"], false);
        assert!(fake.dispatches().is_empty());
    }
}

#[test]
fn a_restore_after_a_placement_that_never_held_places_the_window_and_drops_the_record() {
    let mut fake = world(
        LUA_REPLY,
        vec![
            frame(maximized(), Some(TARGET)),
            frame(tagged(floating_at((40, 60))), Some(TARGET)),
            frame(tagged(floating_at((40, 60))), Some(TARGET)),
            frame(tagged(floating_at((-984, -580))), Some(TARGET)),
            frame(floating_at((-984, -580)), Some(TARGET)),
        ],
    );
    fake.origins = Some(vec![MOVED]);
    window_op_reported(&mut fake, &window(), &WindowOp::Restore).expect_err("unverified");

    let report = window_op_reported(&mut fake, &window(), &WindowOp::Restore).expect("restored");

    assert_eq!(
        report.disposition(),
        DeliverySemantics::delivered_verified()
    );
    assert_eq!(fake.dispatches(), vec![UNSET, PLACE, PLACE, UNTAG]);
    let observed = &report.result.details.expect("details")["observed"];
    assert_eq!(
        (observed["x"].clone(), observed["y"].clone()),
        (json!(-984), json!(-580))
    );
}

#[test]
fn a_restore_with_an_unreadable_record_only_drops_it() {
    let stale = "agent-desktop-restore:x";
    let mut kept = floating_at((40, 60));
    kept.marks.tags = vec![stale.into()];
    let mut fake = world(
        LUA_REPLY,
        vec![
            frame(kept, Some(TARGET)),
            frame(floating_at((40, 60)), Some(TARGET)),
        ],
    );
    fake.origins = None;

    let report = window_op_reported(&mut fake, &window(), &WindowOp::Restore).expect("untagged");

    assert_eq!(
        report.disposition(),
        DeliverySemantics::delivered_verified()
    );
    assert_eq!(
        fake.dispatches(),
        vec![
            "dispatch hl.dsp.window.tag({tag=\"-agent-desktop-restore:x\",window=\"address:0x10\"})"
        ]
    );
}

#[test]
fn a_restore_of_a_window_with_no_record_is_already_satisfied() {
    let mut fake = world(LUA_REPLY, vec![frame(floating_at((40, 60)), Some(HOLDER))]);

    let report = window_op_reported(&mut fake, &window(), &WindowOp::Restore).expect("satisfied");

    assert_eq!(report.disposition(), DeliverySemantics::not_delivered());
    assert_eq!(report.result.action, "restore");
    assert!(fake.dispatches().is_empty());
    assert!(!fake.log.iter().any(|entry| entry == "j/monitors"));
}
