use agent_desktop_core::{AdapterError, Deadline, NativeHandle};

use super::AtspiElement;
use crate::actions::mutation::Corroboration;
use crate::system::SharedBus;

#[derive(Debug, Clone)]
pub(crate) struct NodeView;

#[expect(dead_code, reason = "no caller yet")]
impl NodeView {
    pub(crate) fn role(&self) -> Option<u32> {
        None
    }

    pub(crate) fn state(&self, _bit: u32) -> Option<bool> {
        None
    }

    pub(crate) fn tokens(&self) -> Option<&[String]> {
        None
    }

    pub(crate) fn interfaces(&self) -> Option<&[String]> {
        None
    }

    pub(crate) fn action_names(&self) -> Option<&[String]> {
        None
    }

    pub(crate) fn name(&self) -> Option<&str> {
        None
    }

    pub(crate) fn value(&self) -> Option<&str> {
        None
    }

    pub(crate) fn child_count(&self) -> Option<usize> {
        None
    }

    pub(crate) fn secure(&self) -> bool {
        false
    }

    pub(crate) fn observation(&self) -> Observation {
        Observation
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Observation;

#[expect(dead_code, reason = "no caller yet")]
impl Observation {
    pub(crate) fn changed_since(&self, _before: &Observation) -> Option<bool> {
        None
    }
}

#[expect(dead_code, reason = "no caller yet")]
pub(crate) fn read_node_view(
    _bus: &SharedBus,
    _handle: &NativeHandle,
    _deadline: Deadline,
) -> Result<NodeView, AdapterError> {
    Err(AdapterError::not_supported("read_node_view"))
}

#[expect(dead_code, reason = "no caller yet")]
pub(crate) fn corroborate(
    _bus: &SharedBus,
    _element: &AtspiElement,
    _deadline: Deadline,
) -> Corroboration {
    Corroboration::NotRead
}

#[cfg(test)]
#[path = "action_read_tests.rs"]
mod tests;
