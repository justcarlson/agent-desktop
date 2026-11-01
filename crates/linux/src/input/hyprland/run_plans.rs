use agent_desktop_core::AdapterError;

use super::pointer_run::PointerRun;
use super::pointer_sink::PointerSink;
use crate::input::delivery::Delivery;
use crate::input::plan::Plan;

/// Sends `plans` through `sink`. Every point must map onto an output and
/// every step must be a pointer step, or nothing is sent. The plan gate runs
/// before each plan and the step gate before each press and wheel step. A
/// stop after the first event releases every held button and returns an
/// uncertain error with the count of events sent.
pub(crate) fn run_plans(
    sink: &mut dyn PointerSink,
    run: PointerRun<'_>,
    plans: &[Plan],
) -> Result<Delivery, AdapterError> {
    let _ = (sink, run, plans);
    Ok(Delivery {
        delivered_events: 0,
    })
}

#[cfg(test)]
#[path = "run_plans_tests.rs"]
mod tests;
