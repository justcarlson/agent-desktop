use agent_desktop_core::Deadline;

use super::AtspiElement;
use super::placement::Placement;
use crate::system::SharedBus;

/// Reads the placement of `element` once, inside the deadline.
#[expect(dead_code, reason = "stub the owning lane replaces")]
pub(crate) fn placement(
    _bus: &SharedBus,
    _element: &AtspiElement,
    _deadline: Deadline,
) -> Placement {
    Placement::default()
}
