use std::time::Duration;

use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, DisplayInfo, DragParams, Point, Rect,
};

use super::*;
use crate::input::hyprland::fake_pointer_sink::{FakePointerSink, SinkEvent};
use crate::input::hyprland::output_point::OutputPoint;
use crate::input::hyprland::wheel_axis::WheelAxis;
use crate::input::mouse::drag_plan::plan_drag;
use crate::system::hyprland::cursor_renderer::hold_registry::{mark_lost, publish, withdraw};

const NONCE: &str = "run-plans-hold-lost-after-press";

struct LosesHoldOnPress {
    inner: FakePointerSink,
}

impl PointerSink for LosesHoldOnPress {
    fn motion(&mut self, point: &OutputPoint) -> Result<(), AdapterError> {
        self.inner.motion(point)
    }

    fn button(&mut self, code: u32, down: bool) -> Result<(), AdapterError> {
        self.inner.button(code, down)?;
        if down {
            mark_lost(NONCE);
        }
        Ok(())
    }

    fn wheel(&mut self, axis: WheelAxis, discrete: i32) -> Result<(), AdapterError> {
        self.inner.wheel(axis, discrete)
    }

    fn pause(&mut self, duration: Duration) {
        self.inner.pause(duration);
    }

    fn recover(&mut self, buttons: &[u32]) -> bool {
        self.inner.recover(buttons)
    }

    fn finish(&mut self) -> Result<(), AdapterError> {
        self.inner.finish()
    }

    fn settle(&mut self) -> bool {
        self.inner.settle()
    }
}

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

fn long_drag(deadline: Deadline) -> Vec<Plan> {
    let params = DragParams {
        from: Point { x: 100.0, y: 100.0 },
        to: Point { x: 700.0, y: 400.0 },
        duration_ms: Some(400),
        drop_delay_ms: Some(0),
    };
    vec![plan_drag(&params, deadline).unwrap_or_else(|error| panic!("{error:?}"))]
}

#[test]
fn a_hold_lost_after_the_drag_press_stops_the_motion_and_releases_the_button() {
    let deadline = Deadline::after(5_000).unwrap_or_else(|error| panic!("{error:?}"));
    let displays = displays();
    let plans = long_drag(deadline);
    let mut sink = LosesHoldOnPress {
        inner: FakePointerSink::default(),
    };
    publish(NONCE, vec![7]);
    let outcome = run_plans(
        &mut sink,
        PointerRun {
            displays: &displays,
            plan_gate: &mut || Ok(()),
            step_gate: &mut || Ok(()),
            deadline,
            progress: &Progress::default(),
        },
        &plans,
    );
    withdraw(NONCE);
    let error = outcome
        .err()
        .unwrap_or_else(|| panic!("a drag whose cursor hide lapsed must not succeed"));
    let events = &sink.inner.events;
    let press = events
        .iter()
        .position(|event| matches!(event, SinkEvent::Button(_, true)))
        .unwrap_or_else(|| panic!("the drag never pressed: {events:?}"));
    let after_press = &events[press + 1..];
    assert!(
        matches!(after_press, [SinkEvent::Recover(buttons)] if !buttons.is_empty()),
        "events after the lost hold: {after_press:?}"
    );
    assert_eq!(error.disposition, DeliverySemantics::uncertain());
    let details = error.details.unwrap_or_default();
    assert_eq!(details["kind"], "cursor_hide_renewal_failed");
    assert_eq!(details["buttons_released"], true);
    assert_eq!(details["delivered_events"], press + 1);
}
