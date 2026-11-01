use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode};
use serde_json::json;

use super::pointer_event::PointerEvent;
use super::pointer_run::PointerRun;
use super::pointer_sink::PointerSink;
use crate::input::delivery::Delivery;
use crate::input::plan::Plan;
use crate::input::step::Step;
use crate::input::x11::progress::Progress;

const STOP_SUGGESTION: &str = "Run 'snapshot' to read the real state before retrying: some input may already have been delivered";

/// Sends `plans` through `sink`. Every point must map onto an output and
/// every step must be a pointer step, or nothing is sent. The plan gate runs
/// before each plan and the step gate before each press and wheel step. A
/// stop after the first event releases every held button, confirms every
/// release already sent, and returns an uncertain error with the count of
/// events sent.
pub(crate) fn run_plans(
    sink: &mut dyn PointerSink,
    run: PointerRun<'_>,
    plans: &[Plan],
) -> Result<Delivery, AdapterError> {
    let events = plans
        .iter()
        .map(|plan| {
            plan.steps
                .iter()
                .filter_map(|step| PointerEvent::of(step, run.displays).transpose())
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let PointerRun {
        plan_gate,
        step_gate,
        deadline,
        progress,
        ..
    } = run;
    let mut held: Vec<u32> = Vec::new();
    let mut sent = 0u32;
    let mut unconfirmed = false;
    let outcome = (|| {
        for plan in &events {
            plan_gate()?;
            for event in plan {
                deadline.ensure_remaining()?;
                ensure_not_cancelled(progress)?;
                if event.gated() {
                    step_gate()?;
                }
                event.send(sink)?;
                sent = sent.saturating_add(u32::from(event.counts()));
                event.track(&mut held);
                unconfirmed |= event.releases();
            }
        }
        Ok(())
    })();
    match outcome {
        Ok(()) => Ok(Delivery {
            delivered_events: sent,
            virtual_keyboard: false,
        }),
        Err(error) => {
            let released = match (held.is_empty(), unconfirmed) {
                (false, _) => Some(sink.recover(&held)),
                (true, true) => Some(sink.settle()),
                (true, false) => None,
            };
            Err(stopped(error, sent, released))
        }
    }
}

/// Rejects a step this runner cannot send before any event.
pub(crate) fn unsupported_step(step: &Step) -> AdapterError {
    let what = match step {
        Step::Key { .. } | Step::Keymap(_) | Step::CheckTextState => {
            "modifier keys with pointer input"
        }
        _ => "this pointer button",
    };
    AdapterError::new(
        ErrorCode::ActionNotSupported,
        format!("Hyprland pointer input does not send {what}"),
    )
    .with_details(json!({
        "kind": "hyprland_pointer",
        "cause": "unsupported_step",
        "physical_delivery_started": false,
    }))
    .with_disposition(DeliverySemantics::not_delivered())
}

/// Rejects a point that lies on no output before any event.
pub(crate) fn off_output(x: i32, y: i32) -> AdapterError {
    AdapterError::new(
        ErrorCode::InvalidArgs,
        format!("Point ({x}, {y}) lies on no output of the global layout"),
    )
    .with_details(json!({
        "kind": "hyprland_pointer",
        "cause": "point_off_output",
        "physical_delivery_started": false,
    }))
    .with_disposition(DeliverySemantics::not_delivered())
    .with_suggestion("Run 'list-displays' and pass a point inside one display's bounds")
}

fn ensure_not_cancelled(progress: &Progress) -> Result<(), AdapterError> {
    if !progress.is_cancelled() {
        return Ok(());
    }
    Err(AdapterError::new(
        ErrorCode::ActionFailed,
        "A signal cancelled the pointer input before it finished",
    )
    .with_details(json!({
        "kind": "hyprland_pointer",
        "cause": "cancelled",
        "physical_delivery_started": false,
    })))
}

fn stopped(error: AdapterError, sent: u32, released: Option<bool>) -> AdapterError {
    if sent == 0 {
        return error.with_disposition(DeliverySemantics::not_delivered());
    }
    let mut details = match error.details.clone() {
        Some(serde_json::Value::Object(details)) => details,
        _ => serde_json::Map::new(),
    };
    details.insert("physical_delivery_started".to_owned(), json!(true));
    details.insert("delivered_events".to_owned(), json!(sent));
    if let Some(released) = released {
        details.insert("buttons_released".to_owned(), json!(released));
    }
    error
        .with_details(serde_json::Value::Object(details))
        .with_disposition(DeliverySemantics::uncertain())
        .with_suggestion(STOP_SUGGESTION)
}

#[cfg(test)]
#[path = "run_plans_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "run_plans_confirm_tests.rs"]
mod confirm_tests;
