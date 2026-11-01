use std::sync::Arc;

use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, DisplayInfo, MouseButton, Point, Rect,
};

use super::*;
use crate::input::hyprland::fake_pointer_sink::FakePointerSink;
use crate::input::keyboard::keymap::Keymap;
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
