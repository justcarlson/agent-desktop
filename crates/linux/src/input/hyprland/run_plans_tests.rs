use std::cell::Cell;

use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, DisplayInfo, ErrorCode, MouseButton, Point, Rect,
};

use super::*;
use crate::input::hyprland::fake_pointer_sink::{FakePointerSink, SinkEvent};
use crate::input::hyprland::output_point::OutputPoint;
use crate::input::hyprland::wheel_axis::WheelAxis;
use crate::input::keyboard::keymap::Keymap;
use crate::input::mouse::pointer_plan::plan_click;
use crate::input::mouse::wheel_plan::plan_wheel;
use crate::input::step::Step;
use crate::input::x11::progress::Progress;

const LEFT: u32 = 0x110;

fn displays() -> Vec<DisplayInfo> {
    vec![DisplayInfo {
        id: "Virtual-1".to_owned(),
        bounds: Rect {
            x: 0.0,
            y: 0.0,
            width: 1024.0,
            height: 640.0,
        },
        is_primary: true,
        scale: 1.25,
    }]
}

fn at() -> Point {
    Point { x: 162.0, y: 238.0 }
}

fn motion() -> SinkEvent {
    SinkEvent::Motion(OutputPoint {
        output: "Virtual-1".to_owned(),
        x: 162 * 120,
        y: 238 * 120,
        x_extent: 1024 * 120,
        y_extent: 640 * 120,
    })
}

fn click(count: u32) -> Vec<Plan> {
    let plan = plan_click(&Keymap::default(), at(), &MouseButton::Left, count, &[]);
    vec![plan.unwrap_or_else(|error| panic!("{error:?}"))]
}

fn run(
    sink: &mut FakePointerSink,
    plans: &[Plan],
    step_gate: &mut dyn FnMut() -> Result<(), AdapterError>,
) -> Result<Delivery, AdapterError> {
    let displays = displays();
    run_plans(
        sink,
        PointerRun {
            displays: &displays,
            plan_gate: &mut || Ok(()),
            step_gate,
            deadline: deadline(5_000),
            progress: &Progress::default(),
        },
        plans,
    )
}

#[test]
fn a_triple_click_sends_three_presses_at_the_mapped_point() {
    let mut sink = FakePointerSink::default();
    let delivery = run(&mut sink, &click(3), &mut || Ok(()));
    assert_eq!(delivery.map(|d| d.delivered_events).ok(), Some(7));
    let mut expected = vec![motion()];
    for _ in 0..3 {
        expected.extend([
            SinkEvent::Button(LEFT, true),
            SinkEvent::Button(LEFT, false),
        ]);
    }
    assert_eq!(sink.events, expected);
}

#[test]
fn wheel_plans_send_one_discrete_step_per_click() {
    let mut sink = FakePointerSink::default();
    let plans = plan_wheel(&Keymap::default(), at(), 1.0, 2.0, &[]).unwrap_or_default();
    let delivery = run(&mut sink, &plans, &mut || Ok(()));
    assert!(delivery.is_ok());
    let wheels: Vec<&SinkEvent> = sink
        .events
        .iter()
        .filter(|event| matches!(event, SinkEvent::Wheel(..)))
        .collect();
    assert_eq!(
        wheels,
        vec![
            &SinkEvent::Wheel(WheelAxis::Vertical, -1),
            &SinkEvent::Wheel(WheelAxis::Vertical, -1),
            &SinkEvent::Wheel(WheelAxis::Horizontal, -1),
        ]
    );
}

#[test]
fn the_step_gate_runs_before_each_press_and_wheel_step() {
    let gates = Cell::new(0);
    let mut sink = FakePointerSink::default();
    let _ = run(&mut sink, &click(2), &mut || {
        gates.set(gates.get() + 1);
        Ok(())
    });
    assert_eq!(gates.get(), 2);
    let plans = plan_wheel(&Keymap::default(), at(), 0.0, 3.0, &[]).unwrap_or_default();
    gates.set(0);
    let _ = run(&mut sink, &plans, &mut || {
        gates.set(gates.get() + 1);
        Ok(())
    });
    assert_eq!(gates.get(), 3);
}

#[test]
fn a_point_on_no_output_sends_nothing() {
    let mut sink = FakePointerSink::default();
    let plans = vec![Plan {
        steps: vec![Step::MoveTo { x: 4000, y: 10 }],
        release_origin: None,
    }];
    let error = run(&mut sink, &plans, &mut || Ok(())).err();
    assert_eq!(
        error.as_ref().map(|e| e.code.clone()),
        Some(ErrorCode::InvalidArgs)
    );
    assert_eq!(
        error.map(|e| e.disposition),
        Some(DeliverySemantics::not_delivered())
    );
    assert!(sink.events.is_empty());
}

#[test]
fn a_key_step_is_refused_before_any_event() {
    let mut sink = FakePointerSink::default();
    let mut plans = click(1);
    plans[0].steps.push(Step::Key {
        code: 50,
        down: true,
    });
    let error = run(&mut sink, &plans, &mut || Ok(())).err();
    assert_eq!(
        error.map(|e| e.disposition),
        Some(DeliverySemantics::not_delivered())
    );
    assert!(sink.events.is_empty());
}

#[test]
fn a_focus_change_mid_sequence_stops_uncertain_with_counts() {
    let gates = Cell::new(0);
    let mut sink = FakePointerSink::default();
    let error = run(&mut sink, &click(3), &mut || {
        gates.set(gates.get() + 1);
        if gates.get() == 3 {
            return Err(AdapterError::new(ErrorCode::ActionFailed, "focus changed"));
        }
        Ok(())
    })
    .err();
    let error = error.unwrap_or_else(|| panic!("the run must stop"));
    assert_eq!(error.disposition, DeliverySemantics::uncertain());
    let details = error.details.unwrap_or_default();
    assert_eq!(details["delivered_events"], 5);
    assert_eq!(details["physical_delivery_started"], true);
    assert_eq!(sink.events.len(), 5);
}

#[test]
fn a_refusal_after_the_motion_is_uncertain() {
    let mut sink = FakePointerSink::default();
    let error = run(&mut sink, &click(1), &mut || {
        Err(AdapterError::new(ErrorCode::ActionFailed, "focus changed"))
    })
    .err();
    let details = error
        .as_ref()
        .and_then(|e| e.details.clone())
        .unwrap_or_default();
    assert_eq!(details["delivered_events"], 1);
    assert_eq!(
        error.map(|e| e.disposition),
        Some(DeliverySemantics::uncertain())
    );
}

#[test]
fn a_failure_while_a_button_is_held_releases_it() {
    let mut sink = FakePointerSink {
        fail_at: Some(2),
        ..FakePointerSink::default()
    };
    let error = run(&mut sink, &click(1), &mut || Ok(())).err();
    assert!(error.is_some());
    assert_eq!(
        sink.events,
        vec![
            motion(),
            SinkEvent::Button(LEFT, true),
            SinkEvent::Recover(vec![LEFT]),
        ]
    );
    let details = error.and_then(|e| e.details).unwrap_or_default();
    assert_eq!(details["buttons_released"], true);
}

#[test]
fn an_expired_deadline_mid_run_releases_and_times_out() {
    let mut sink = FakePointerSink::default();
    let displays = displays();
    let expired = deadline(1);
    std::thread::sleep(std::time::Duration::from_millis(5));
    let error = run_plans(
        &mut sink,
        PointerRun {
            displays: &displays,
            plan_gate: &mut || Ok(()),
            step_gate: &mut || Ok(()),
            deadline: expired,
            progress: &Progress::default(),
        },
        &click(1),
    )
    .err();
    assert_eq!(error.map(|e| e.code), Some(ErrorCode::Timeout));
    assert!(sink.events.is_empty());
}

#[test]
fn a_cancel_mid_run_stops_later_events_and_releases_the_held_button() {
    let displays = displays();
    let progress = std::sync::Arc::new(Progress::default());
    let mut sink = FakePointerSink {
        cancel_after: Some((2, std::sync::Arc::clone(&progress))),
        ..FakePointerSink::default()
    };
    let error = run_plans(
        &mut sink,
        PointerRun {
            displays: &displays,
            plan_gate: &mut || Ok(()),
            step_gate: &mut || Ok(()),
            deadline: deadline(5_000),
            progress: &progress,
        },
        &click(2),
    )
    .err()
    .unwrap_or_else(|| panic!("a cancelled run must stop"));
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::uncertain());
    let details = error.details.unwrap_or_default();
    assert_eq!(details["kind"], "hyprland_pointer");
    assert_eq!(details["cause"], "cancelled");
    assert_eq!(details["delivered_events"], 2);
    assert_eq!(details["buttons_released"], true);
    assert_eq!(
        sink.events,
        vec![
            motion(),
            SinkEvent::Button(LEFT, true),
            SinkEvent::Recover(vec![LEFT]),
        ]
    );
}

#[test]
fn a_run_cancelled_before_it_starts_sends_nothing() {
    let mut sink = FakePointerSink::default();
    let displays = displays();
    let progress = Progress::default();
    progress.cancel();
    let error = run_plans(
        &mut sink,
        PointerRun {
            displays: &displays,
            plan_gate: &mut || Ok(()),
            step_gate: &mut || Ok(()),
            deadline: deadline(5_000),
            progress: &progress,
        },
        &click(1),
    )
    .err();
    assert_eq!(
        error.map(|e| e.disposition),
        Some(DeliverySemantics::not_delivered())
    );
    assert!(sink.events.is_empty());
}

fn deadline(timeout_ms: u64) -> Deadline {
    Deadline::after(timeout_ms).unwrap_or_else(|error| panic!("{error:?}"))
}
