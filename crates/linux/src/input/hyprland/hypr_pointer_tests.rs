use agent_desktop_core::{
    ErrorCode, Modifier, MouseButton, MouseEvent, MouseEventKind, OperationReport, Point,
};

use super::{coordinate_report, event_plans, while_held};
use crate::input::delivery::Delivery;
use crate::input::step::Step;

fn event(kind: MouseEventKind, modifiers: Vec<Modifier>) -> MouseEvent {
    MouseEvent {
        kind,
        point: Point { x: 10.0, y: 20.0 },
        button: MouseButton::Left,
        modifiers,
    }
}

#[test]
fn a_modifier_is_refused_before_any_event() {
    let error = event_plans(&event(
        MouseEventKind::Click { count: 1 },
        vec![Modifier::Ctrl],
    ))
    .unwrap_err();

    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    let details = error.details.unwrap_or_default();
    assert_eq!(details["cause"], "modifiers_unsupported");
    assert_eq!(details["physical_delivery_started"], false);
}

#[test]
fn a_standalone_press_or_release_is_refused() {
    assert!(event_plans(&event(MouseEventKind::Down, Vec::new())).is_err());
    assert!(event_plans(&event(MouseEventKind::Up, Vec::new())).is_err());
}

#[test]
fn a_triple_click_plans_three_presses_and_no_key_step() {
    let plans = event_plans(&event(MouseEventKind::Click { count: 3 }, Vec::new()))
        .unwrap_or_else(|error| panic!("{error:?}"));
    let steps: Vec<&Step> = plans.iter().flat_map(|plan| plan.steps.iter()).collect();

    let presses = steps
        .iter()
        .filter(|step| matches!(step, Step::Button { down: true, .. }))
        .count();
    assert_eq!(presses, 3);
    assert!(
        !steps
            .iter()
            .any(|step| matches!(step, Step::Key { .. } | Step::Keymap(_)))
    );
}

#[test]
fn a_coordinate_report_carries_no_receiver_proof() {
    let report = coordinate_report(
        OperationReport::pointer_dispatched(&event(MouseEventKind::Move, Vec::new()), false),
        &Delivery {
            delivered_events: 1,
            virtual_keyboard: false,
        },
    );

    let details = report.result.details.unwrap_or_default();
    assert_eq!(details["receiver_proof"], false);
    assert_eq!(details["delivered_events"], 1);
}

#[test]
fn a_held_gate_runs_the_wrapped_gate_while_the_hide_holds() {
    let nonce = "pointer-gate-held";
    crate::system::hyprland::cursor_renderer::hold_registry::publish(nonce, vec![7]);
    let mut calls = 0;
    let mut inner = || {
        calls += 1;
        Ok(())
    };
    let outcome = while_held(&mut inner)();
    crate::system::hyprland::cursor_renderer::hold_registry::withdraw(nonce);
    assert!(outcome.is_ok());
    assert_eq!(calls, 1);
}

#[test]
fn a_lost_hide_stops_the_gate_before_the_wrapped_gate_runs() {
    let nonce = "pointer-gate-lost";
    crate::system::hyprland::cursor_renderer::hold_registry::publish(nonce, vec![7]);
    crate::system::hyprland::cursor_renderer::hold_registry::mark_lost(nonce);
    let mut calls = 0;
    let mut inner = || {
        calls += 1;
        Ok(())
    };
    let outcome = while_held(&mut inner)();
    crate::system::hyprland::cursor_renderer::hold_registry::withdraw(nonce);
    let error = outcome.expect_err("lost");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(
        error.details.unwrap_or_default()["kind"],
        "cursor_hide_renewal_failed"
    );
    assert_eq!(calls, 0);
}
