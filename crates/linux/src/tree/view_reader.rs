use agent_desktop_core::{AdapterError, Deadline};
use zbus::Connection;

use super::AtspiElement;
use super::live_read::NodeReader;
use super::node_evidence::{NodeEvidence, read_evidence};
use super::placement::Placement;
use super::resolve_errors::stopped_app;
use crate::system::within;

pub(super) struct ViewReader<'a> {
    pub(super) connection: &'a Connection,
    pub(super) deadline: Deadline,
}

impl NodeReader for ViewReader<'_> {
    fn read(&mut self, element: &AtspiElement) -> Result<NodeEvidence, AdapterError> {
        within(self.deadline, read_evidence(self.connection, element))
    }

    fn stopped_app(&mut self, pid: u32) -> Option<String> {
        stopped_app(pid)
    }

    fn placement(&mut self, _element: &AtspiElement, _deadline: Deadline) -> Placement {
        Placement::default()
    }

    fn global_position(&mut self, _element: &AtspiElement, _deadline: Deadline) -> bool {
        false
    }
}
