use agent_desktop_core::{AdapterError, Deadline, SignalBaseline, SignalFilter};

use super::SharedBus;

pub(crate) fn capture_signal_baseline(
    _bus: &SharedBus,
    _filter: &SignalFilter,
    _deadline: Deadline,
) -> Result<SignalBaseline, AdapterError> {
    Err(AdapterError::not_supported("capture_signal_baseline"))
}

#[cfg(test)]
#[path = "signal_capture_tests.rs"]
mod tests;
