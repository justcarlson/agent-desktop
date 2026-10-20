use agent_desktop_core::{
    AdapterError, Deadline, InteractionLease, InteractionPolicy, MouseButton, Point, WindowInfo,
};

use crate::system::SharedBus;
use crate::system::shell::container::{Landmark, locate};
use crate::system::shell::identity::find_root;
use crate::system::shell::kinds::ShellKind;
use crate::system::shell::open::{Opened, close, is_showing, open};
use crate::tree::role_numbers::{LABEL, NOTIFICATION, PUSH_BUTTON};
use crate::tree::{
    AtspiElement, Extents, Field, ObservedRead, Walk, WalkLimits, WalkedNode, walk_tree,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    Notification,
    Label,
    PushButton,
    Other,
}

#[derive(Debug, Clone)]
pub(crate) struct Node {
    pub(crate) depth: usize,
    pub(crate) role: Role,
    pub(crate) name: String,
    pub(crate) extents: Option<Extents>,
    pub(crate) element: AtspiElement,
}

/// The nodes under a container in document order. `complete` is false when
/// the read was cut short or a role or a name could not be read.
#[derive(Debug, Clone)]
pub(crate) struct Reading {
    pub(crate) nodes: Vec<Node>,
    pub(crate) complete: bool,
}

/// What the notification code needs from the shell core, so its tests run on fakes.
pub(crate) trait ShellWorld {
    fn require_daemon(&self, deadline: Deadline) -> Result<(), AdapterError>;
    fn locate(&self, landmark: Landmark, deadline: Deadline) -> Result<AtspiElement, AdapterError>;
    fn read(&self, root: &AtspiElement, deadline: Deadline) -> Result<Reading, AdapterError>;
    fn open(
        &self,
        policy: InteractionPolicy,
        lease: &InteractionLease,
    ) -> Result<(WindowInfo, Opened), AdapterError>;
    fn close(&self, lease: &InteractionLease) -> Result<(), AdapterError>;
    fn is_showing(&self, deadline: Deadline) -> Result<bool, AdapterError>;
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
    fn require_daemon(&self, deadline: Deadline) -> Result<(), AdapterError> {
        crate::system::shell::detect::require_daemon(deadline)
    }

    fn locate(&self, landmark: Landmark, deadline: Deadline) -> Result<AtspiElement, AdapterError> {
        let root = find_root(self.bus, deadline)?;
        locate(self.bus, &root, landmark, deadline)
    }

    fn read(&self, root: &AtspiElement, deadline: Deadline) -> Result<Reading, AdapterError> {
        let walk = walk_tree(self.bus, root, &WalkLimits::default(), deadline)?;
        Ok(project(&walk))
    }

    fn open(
        &self,
        policy: InteractionPolicy,
        lease: &InteractionLease,
    ) -> Result<(WindowInfo, Opened), AdapterError> {
        open(self.bus, ShellKind::MessageList, policy, lease)
    }

    fn close(&self, lease: &InteractionLease) -> Result<(), AdapterError> {
        close(self.bus, ShellKind::MessageList, lease)
    }

    fn is_showing(&self, deadline: Deadline) -> Result<bool, AdapterError> {
        is_showing(self.bus, ShellKind::MessageList, deadline)
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

pub(crate) fn project(walk: &Walk<ObservedRead>) -> Reading {
    let mut nodes = Vec::new();
    let mut complete = walk.complete;
    flatten(&walk.root, 0, &mut nodes, &mut complete);
    Reading { nodes, complete }
}

fn flatten(
    walked: &WalkedNode<ObservedRead>,
    depth: usize,
    nodes: &mut Vec<Node>,
    complete: &mut bool,
) {
    let raw = &walked.payload.node;
    if raw.traits.role.is_failed() || raw.properties.name.is_failed() {
        *complete = false;
    }
    nodes.push(Node {
        depth,
        role: role_of(raw.traits.role.present().copied()),
        name: name_of(&raw.properties.name),
        extents: raw.extents.present().copied(),
        element: raw.element.clone(),
    });
    for child in &walked.children {
        flatten(child, depth + 1, nodes, complete);
    }
}

fn role_of(number: Option<u32>) -> Role {
    match number {
        Some(NOTIFICATION) => Role::Notification,
        Some(LABEL) => Role::Label,
        Some(PUSH_BUTTON) => Role::PushButton,
        _ => Role::Other,
    }
}

fn name_of(name: &Field<String>) -> String {
    name.present().cloned().unwrap_or_default()
}

#[cfg(test)]
#[path = "world_tests.rs"]
mod tests;
