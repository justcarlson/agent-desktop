#![expect(dead_code, reason = "stub the owning lane replaces")]

use agent_desktop_core::{
    AdapterError, Deadline, InteractionLease, InteractionPolicy, MouseButton, Point,
};

use crate::system::SharedBus;
use crate::system::shell::container::{Landmark, locate};
use crate::system::shell::identity::find_root;
use crate::tree::AtspiElement;

/// What the tray code needs from the shell core, so its tests run on fakes.
pub(crate) trait ShellWorld {
    fn locate(&self, landmark: Landmark, deadline: Deadline) -> Result<AtspiElement, AdapterError>;
    fn click(
        &self,
        node: &AtspiElement,
        button: MouseButton,
        policy: InteractionPolicy,
        lease: &InteractionLease,
    ) -> Result<Point, AdapterError>;
}

pub(crate) struct LiveWorld<'a> {
    bus: &'a SharedBus,
}

impl<'a> LiveWorld<'a> {
    pub(crate) fn new(bus: &'a SharedBus) -> Self {
        Self { bus }
    }
}

impl ShellWorld for LiveWorld<'_> {
    fn locate(&self, landmark: Landmark, deadline: Deadline) -> Result<AtspiElement, AdapterError> {
        let root = find_root(self.bus, deadline)?;
        locate(self.bus, &root, landmark, deadline)
    }

    fn click(
        &self,
        node: &AtspiElement,
        button: MouseButton,
        policy: InteractionPolicy,
        lease: &InteractionLease,
    ) -> Result<Point, AdapterError> {
        crate::input::shell_click::click(self.bus, node, button, policy, lease)
    }
}

#[cfg(test)]
#[path = "world_tests.rs"]
mod tests;
