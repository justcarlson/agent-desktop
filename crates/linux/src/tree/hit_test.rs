use agent_desktop_core::{AdapterError, Deadline, HitTestResult, NativeHandle, Point};

use crate::system::SharedBus;

pub(crate) fn hit_test(
    _bus: &SharedBus,
    _handle: &NativeHandle,
    _point: Point,
    _deadline: Deadline,
) -> Result<HitTestResult, AdapterError> {
    Err(AdapterError::not_supported("hit_test"))
}
