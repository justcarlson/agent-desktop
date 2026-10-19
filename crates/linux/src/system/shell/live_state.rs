use agent_desktop_core::Deadline;

use crate::system::SharedBus;
use crate::tree::AtspiElement;
use crate::tree::placement::Placement;

pub(crate) fn placement(
    _bus: &SharedBus,
    _element: &AtspiElement,
    _deadline: Deadline,
) -> Placement {
    Placement::default()
}

#[cfg(test)]
#[path = "live_state_tests.rs"]
mod tests;
