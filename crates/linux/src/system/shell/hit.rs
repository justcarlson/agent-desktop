use agent_desktop_core::{AdapterError, Deadline, HitTestResult, Point};

use crate::system::SharedBus;
use crate::tree::AtspiElement;

pub(crate) fn hit_test(
    _bus: &SharedBus,
    _target: &AtspiElement,
    _point: Point,
    _deadline: Deadline,
) -> Result<HitTestResult, AdapterError> {
    Ok(HitTestResult::Unknown)
}

#[cfg(test)]
#[path = "hit_tests.rs"]
mod tests;
