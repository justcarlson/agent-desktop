use agent_desktop_core::{DeliverySemantics, ErrorCode, WindowOp};

use super::*;
use crate::system::hyprland::window_op::step_goal::StepGoal;

const TARGET: u64 = 0x10;
const HOLDER: u64 = 0x30;
const LUA: ConfigDialect = ConfigDialect::Lua;
const LEGACY: ConfigDialect = ConfigDialect::Legacy;

fn floating() -> ClientState {
    ClientState {
        address: TARGET,
        at: (40, 60),
        size: (630, 755),
        floating: true,
        fullscreen: 0,
        workspace: "1".into(),
    }
}

fn with(change: impl FnOnce(&mut ClientState)) -> ClientState {
    let mut state = floating();
    change(&mut state);
    state
}

fn cause(result: Result<Vec<WindowStep>, AdapterError>) -> String {
    let error = result.expect_err("refused");
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    let details = error.details.expect("details");
    assert_eq!(details["physical_delivery_started"], false);
    details["cause"].as_str().expect("cause").to_owned()
}

fn moved(x: f64, y: f64) -> WindowOp {
    WindowOp::Move { x, y }
}

fn resized(width: f64, height: f64) -> WindowOp {
    WindowOp::Resize { width, height }
}

#[test]
fn a_tiled_window_is_neither_moved_nor_resized() {
    let tiled = with(|state| state.floating = false);
    for op in [moved(100.0, 100.0), resized(500.0, 400.0)] {
        assert_eq!(cause(plan(&op, &tiled, Some(HOLDER), LUA)), "tiled_window");
    }
}

#[test]
fn a_maximized_or_fullscreen_window_is_neither_moved_nor_resized() {
    for mode in [1, 2] {
        let filling = with(|state| state.fullscreen = mode);
        for op in [moved(100.0, 100.0), resized(500.0, 400.0)] {
            assert_eq!(cause(plan(&op, &filling, None, LUA)), "window_maximized");
        }
    }
}

#[test]
fn a_minimized_window_is_neither_moved_resized_nor_maximized() {
    let parked = with(|state| state.workspace = "special:agent-desktop-minimized-1".into());
    for op in [moved(1.0, 1.0), resized(500.0, 400.0), WindowOp::Maximize] {
        assert_eq!(cause(plan(&op, &parked, None, LUA)), "window_minimized");
    }
}

#[test]
fn a_move_is_one_step_to_the_rounded_point() {
    let steps = plan(&moved(-49.6, 20.2), &floating(), Some(HOLDER), LUA).expect("planned");

    assert_eq!(
        steps,
        vec![WindowStep::new(
            "dispatch hl.dsp.window.move({x=-50,y=20,window=\"address:0x10\"})".into(),
            vec![StepGoal::At(-50, 20)],
        )]
    );
}

#[test]
fn a_resize_moves_the_window_back_to_its_old_corner() {
    let steps = plan(&resized(500.0, 400.0), &floating(), Some(HOLDER), LUA).expect("planned");

    assert_eq!(
        steps,
        vec![
            WindowStep::new(
                "dispatch hl.dsp.window.resize({x=500,y=400,window=\"address:0x10\"})".into(),
                vec![StepGoal::Resized {
                    wanted: (500, 400),
                    from: (630, 755),
                }],
            ),
            WindowStep::new(
                "dispatch hl.dsp.window.move({x=40,y=60,window=\"address:0x10\"})".into(),
                vec![StepGoal::At(40, 60), StepGoal::Size(500, 400)],
            ),
        ]
    );
}

#[test]
fn maximize_never_takes_focus_from_another_window() {
    let tiled = with(|state| state.floating = false);
    assert_eq!(
        cause(plan(&WindowOp::Maximize, &tiled, Some(HOLDER), LUA)),
        "maximize_takes_focus"
    );
    for active in [Some(TARGET), None] {
        let steps = plan(&WindowOp::Maximize, &tiled, active, LUA).expect("planned");
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].goals, vec![StepGoal::Fullscreen(1)]);
    }
}

#[test]
fn maximize_of_a_fullscreen_window_is_refused() {
    let fullscreen = with(|state| state.fullscreen = 2);
    assert_eq!(
        cause(plan(&WindowOp::Maximize, &fullscreen, None, LUA)),
        "window_fullscreen"
    );
}

#[test]
fn a_legacy_config_cannot_aim_maximize_or_unmaximize() {
    assert_eq!(
        cause(plan(&WindowOp::Maximize, &floating(), Some(TARGET), LEGACY)),
        "legacy_config_focused_only"
    );
    let maximized = with(|state| state.fullscreen = 1);
    assert_eq!(
        cause(plan(&WindowOp::Restore, &maximized, Some(TARGET), LEGACY)),
        "legacy_config_focused_only"
    );
}

#[test]
fn minimize_parks_the_window_on_a_special_workspace_named_for_its_origin() {
    let steps = plan(&WindowOp::Minimize, &floating(), Some(TARGET), LUA).expect("planned");

    assert_eq!(
        steps,
        vec![WindowStep::new(
            "dispatch hl.dsp.window.move({workspace=\"special:agent-desktop-minimized-1\",follow=false,window=\"address:0x10\"})".into(),
            vec![StepGoal::Workspace("special:agent-desktop-minimized-1".into())],
        )]
    );
}

#[test]
fn an_origin_that_cannot_be_named_is_refused() {
    for origin in ["special:scratch", "two words", ""] {
        let state = with(|state| state.workspace = origin.into());
        assert_eq!(
            cause(plan(&WindowOp::Minimize, &state, None, LUA)),
            "workspace_name_unsupported"
        );
    }
}

#[test]
fn restore_returns_the_window_then_clears_its_maximized_state() {
    let parked = with(|state| {
        state.workspace = "special:agent-desktop-minimized-web".into();
        state.fullscreen = 1;
    });

    let steps = plan(&WindowOp::Restore, &parked, Some(HOLDER), LUA).expect("planned");

    assert_eq!(
        steps,
        vec![
            WindowStep::new(
                "dispatch hl.dsp.window.move({workspace=\"name:web\",follow=false,window=\"address:0x10\"})".into(),
                vec![StepGoal::Workspace("web".into())],
            ),
            WindowStep::new(
                "dispatch hl.dsp.window.fullscreen_state({internal=0,client=0,action=\"set\",window=\"address:0x10\"})".into(),
                vec![StepGoal::Fullscreen(0)],
            ),
        ]
    );
}

#[test]
fn a_legacy_restore_of_a_minimized_window_moves_it_back() {
    let parked = with(|state| state.workspace = "special:agent-desktop-minimized-1".into());

    let steps = plan(&WindowOp::Restore, &parked, None, LEGACY).expect("planned");

    assert_eq!(
        steps,
        vec![WindowStep::new(
            "dispatch movetoworkspacesilent 1,address:0x10".into(),
            vec![StepGoal::Workspace("1".into())],
        )]
    );
}

#[test]
fn an_operation_already_satisfied_plans_no_step() {
    let parked = with(|state| state.workspace = "special:agent-desktop-minimized-1".into());
    let maximized = with(|state| state.fullscreen = 1);
    let cases = [
        (moved(41.0, 59.0), floating()),
        (resized(631.0, 754.0), floating()),
        (WindowOp::Minimize, parked),
        (WindowOp::Maximize, maximized),
        (WindowOp::Restore, floating()),
    ];
    for (op, state) in cases {
        assert_eq!(
            plan(&op, &state, Some(HOLDER), LUA).expect("planned"),
            vec![]
        );
    }
}
