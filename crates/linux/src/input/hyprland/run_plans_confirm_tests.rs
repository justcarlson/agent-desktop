use std::sync::Arc;

use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, DisplayInfo, DragParams, ErrorCode, MouseButton,
    Point, Rect,
};

use super::*;
use crate::input::hyprland::fake_pointer_sink::FakePointerSink;
use crate::input::keyboard::keymap::Keymap;
use crate::input::mouse::drag_plan::plan_drag;
use crate::input::mouse::pointer_plan::plan_click;
use crate::input::x11::progress::Progress;

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
        scale: 1.0,
    }]
}

fn deadline() -> Deadline {
    Deadline::after(5_000).unwrap_or_else(|error| panic!("{error:?}"))
}

fn click(count: u32) -> Vec<Plan> {
    let at = Point { x: 162.0, y: 238.0 };
    let plan = plan_click(&Keymap::default(), at, &MouseButton::Left, count, &[]);
    vec![plan.unwrap_or_else(|error| panic!("{error:?}"))]
}

fn drag() -> Vec<Plan> {
    let params = DragParams {
        from: Point { x: 100.0, y: 100.0 },
        to: Point { x: 300.0, y: 120.0 },
        duration_ms: Some(100),
        drop_delay_ms: Some(0),
    };
    vec![plan_drag(&params, deadline()).unwrap_or_else(|error| panic!("{error:?}"))]
}

fn run(
    sink: &mut FakePointerSink,
    progress: &Progress,
    plans: &[Plan],
) -> Result<Delivery, AdapterError> {
    let displays = displays();
    run_plans(
        sink,
        PointerRun {
            displays: &displays,
            plan_gate: &mut || Ok(()),
            step_gate: &mut || Ok(()),
            deadline: deadline(),
            progress,
        },
        plans,
    )
}

fn timed_out_finish() -> FakePointerSink {
    FakePointerSink {
        finish_error: Some(ErrorCode::Timeout),
        ..FakePointerSink::default()
    }
}

fn assert_sent_but_unconfirmed(error: AdapterError, sink: &FakePointerSink) {
    assert_eq!(error.disposition, DeliverySemantics::uncertain());
    assert_eq!(error.suggestion.as_deref(), Some(STOP_SUGGESTION));
    let details = error.details.unwrap_or_default();
    assert_eq!(details["physical_delivery_started"], true);
    assert_eq!(details["delivered_events"], sink.events.len());
    assert_eq!(details["buttons_released"], true);
    assert_eq!(sink.settled, 1, "the releases were not confirmed");
}

#[test]
fn a_cancel_in_the_pause_between_clicks_confirms_the_release_already_sent() {
    let progress = Arc::new(Progress::default());
    let mut sink = FakePointerSink {
        cancel_after: Some((3, Arc::clone(&progress))),
        ..FakePointerSink::default()
    };
    let error = run(&mut sink, &progress, &click(2))
        .err()
        .unwrap_or_else(|| panic!("a cancelled run must stop"));
    assert_eq!(sink.events.len(), 3);
    assert_eq!(
        error.details.clone().unwrap_or_default()["cause"],
        "cancelled"
    );
    assert_sent_but_unconfirmed(error, &sink);
}

#[test]
fn a_final_roundtrip_timeout_after_a_click_is_uncertain_with_its_count() {
    let mut sink = timed_out_finish();
    let error = run(&mut sink, &Progress::default(), &click(1))
        .err()
        .unwrap_or_else(|| panic!("an unconfirmed click must not succeed"));
    assert_eq!(sink.events.len(), 3);
    assert_sent_but_unconfirmed(error, &sink);
}

#[test]
fn a_final_roundtrip_timeout_after_a_drag_is_uncertain_with_its_count() {
    let mut sink = timed_out_finish();
    let error = run(&mut sink, &Progress::default(), &drag())
        .err()
        .unwrap_or_else(|| panic!("an unconfirmed drag must not succeed"));
    assert!(sink.events.len() > 3);
    assert_sent_but_unconfirmed(error, &sink);
}

#[test]
fn a_confirmed_run_needs_no_separate_release_settle() {
    let mut sink = FakePointerSink::default();
    let delivery = run(&mut sink, &Progress::default(), &click(2));
    assert_eq!(delivery.map(|d| d.delivered_events).ok(), Some(5));
    assert_eq!((sink.finished, sink.settled), (1, 0));
}
