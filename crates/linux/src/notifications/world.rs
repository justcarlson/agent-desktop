use agent_desktop_core::{
    AdapterError, Deadline, ErrorCode, InteractionLease, InteractionPolicy, MouseButton, Point,
    WindowInfo,
};

use crate::system::SharedBus;
use crate::system::shell::anchor::Anchor;
use crate::system::shell::container::{Landmark, locate_anchor};
use crate::system::shell::identity::find_root;
use crate::system::shell::kinds::ShellKind;
use crate::system::shell::light::{Light, LiveNodes, Nodes};
use crate::system::shell::open::{Opened, close, is_showing, open};
use crate::tree::role_numbers::{LABEL, NOTIFICATION, PUSH_BUTTON};
use crate::tree::{AtspiElement, Extents};

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
        let located = locate_anchor(self.bus, &root, Anchor::Landmark(landmark), deadline)?;
        Ok(located.container.element)
    }

    fn read(&self, root: &AtspiElement, deadline: Deadline) -> Result<Reading, AdapterError> {
        let nodes = LiveNodes::new(self.bus.get(deadline)?.inner().clone(), deadline);
        read_subtree(&nodes, root, deadline)
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
        crate::input::target_pointer::click_shell(self.bus, node, button, policy, lease)
    }
}

const MAX_NODES: usize = 4_000;

/// Reads the subtree under `root` level by level and flattens it in
/// document order. A node that vanishes mid-read ends the reading as
/// incomplete rather than failing it.
pub(crate) fn read_subtree<N: Nodes>(
    nodes: &N,
    root: &AtspiElement,
    deadline: Deadline,
) -> Result<Reading, AdapterError> {
    let mut arena: Vec<(Light, Vec<usize>)> = Vec::new();
    let mut complete = true;
    let mut frontier: Vec<(AtspiElement, Option<usize>)> = vec![(root.clone(), None)];
    while !frontier.is_empty() {
        if deadline.is_expired() {
            return Err(deadline.timeout_error());
        }
        let elements: Vec<AtspiElement> = frontier
            .iter()
            .map(|(element, _)| element.clone())
            .collect();
        let reads = nodes.read_level(&elements);
        let mut next = Vec::new();
        for ((_, parent), read) in frontier.drain(..).zip(reads) {
            let light = match read {
                Ok(light) => light,
                Err(error) if error.code == ErrorCode::ElementNotFound => {
                    complete = false;
                    continue;
                }
                Err(error) => return Err(error),
            };
            let index = arena.len();
            let children = light.children.clone();
            arena.push((light, Vec::new()));
            if let Some(parent) = parent {
                arena[parent].1.push(index);
            }
            next.extend(children.into_iter().map(|child| (child, Some(index))));
        }
        if arena.len() > MAX_NODES {
            complete = false;
            break;
        }
        frontier = next;
    }
    let mut flat = Vec::with_capacity(arena.len());
    if !arena.is_empty() {
        flatten(&arena, 0, 0, &mut flat);
    }
    Ok(Reading {
        nodes: flat,
        complete,
    })
}

fn flatten(arena: &[(Light, Vec<usize>)], index: usize, depth: usize, out: &mut Vec<Node>) {
    let (light, children) = &arena[index];
    out.push(Node {
        depth,
        role: role_of(light.role),
        name: light.name.clone(),
        extents: light.extents,
        element: light.element.clone(),
    });
    for child in children {
        flatten(arena, *child, depth + 1, out);
    }
}

fn role_of(number: u32) -> Role {
    match number {
        NOTIFICATION => Role::Notification,
        LABEL => Role::Label,
        PUSH_BUTTON => Role::PushButton,
        _ => Role::Other,
    }
}

#[cfg(test)]
#[path = "world_tests.rs"]
mod tests;
