use agent_desktop_core::Deadline;

use super::{AtspiElement, Extents};
use crate::system::SharedBus;

/// An element's rectangle in window coordinates, or `None` when it cannot be
/// read.
#[expect(dead_code, reason = "stub the owning lane replaces")]
pub(crate) fn read_window_extents(
    _bus: &SharedBus,
    _element: &AtspiElement,
    _deadline: Deadline,
) -> Option<Extents> {
    None
}
