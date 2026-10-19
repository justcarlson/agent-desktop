use std::ops::Range;

use agent_desktop_core::{AdapterError, Deadline, NativeHandle};

use super::AtspiElement;
use crate::system::SharedBus;

pub(crate) type CaretAndSelection = (usize, Vec<Range<usize>>);

pub(crate) fn read_selection(
    _bus: &SharedBus,
    _handle: &NativeHandle,
    _deadline: Deadline,
) -> Result<Option<Range<usize>>, AdapterError> {
    Ok(None)
}

#[expect(dead_code, reason = "no caller yet")]
pub(crate) fn read_caret_and_selection(
    _bus: &SharedBus,
    _element: &AtspiElement,
    _deadline: Deadline,
) -> Result<Option<CaretAndSelection>, AdapterError> {
    Ok(None)
}

#[cfg(test)]
#[path = "text_selection_tests.rs"]
mod tests;
