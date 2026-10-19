use agent_desktop_core::{AdapterError, Deadline, NativeHandle, state};
use zbus::Connection;

use super::absence::{Failure, failure};
use super::live_read::{NodeReader, read_live};
use super::node_evidence::{NodeEvidence, read_evidence};
use super::node_read::call;
use super::placement::Placement;
use super::resolve_errors::stopped_app;
use super::state_bits::{CHECKED, EXPANDED, PRESSED, SELECTED, StateBits};
use super::{AtspiElement, Field};
use crate::actions::mutation::Corroboration;
use crate::system::{ACCESSIBLE_INTERFACE, SharedBus, within};

/// One node read for an action: the evidence the snapshot walk projects, kept
/// whole so a write and its read-back look at the node the same way.
#[derive(Debug, Clone)]
pub(crate) struct NodeView {
    node: NodeEvidence,
}

fn confirmed(field: &Field<Vec<String>>) -> Option<&[String]> {
    match field {
        Field::Present(list) => Some(list),
        Field::Absent => Some(&[]),
        Field::Failed(_) => None,
    }
}

fn text_of(field: &Field<String>) -> Option<Option<String>> {
    match field {
        Field::Present(text) => Some(Some(text.clone())),
        Field::Absent => Some(None),
        Field::Failed(_) => None,
    }
}

impl NodeView {
    pub(crate) fn new(node: NodeEvidence) -> Self {
        Self { node }
    }

    pub(crate) fn role(&self) -> Option<u32> {
        self.node.read.node.traits.role.present().copied()
    }

    /// Whether a state bit is set, or `None` when the state read failed.
    pub(crate) fn state(&self, bit: u32) -> Option<bool> {
        self.node
            .read
            .node
            .traits
            .states
            .present()
            .map(|words| StateBits::new(words).has(bit))
    }

    pub(crate) fn tokens(&self) -> Option<&[String]> {
        self.node.evidence.states.known().map(Vec::as_slice)
    }

    pub(crate) fn interfaces(&self) -> Option<&[String]> {
        confirmed(&self.node.read.node.traits.interfaces)
    }

    pub(crate) fn action_names(&self) -> Option<&[String]> {
        confirmed(&self.node.read.node.actions)
    }

    pub(crate) fn name(&self) -> Option<&str> {
        self.node
            .read
            .node
            .properties
            .name
            .present()
            .map(String::as_str)
    }

    /// The node's value, withheld from a secure field whatever was read.
    pub(crate) fn value(&self) -> Option<&str> {
        if self.secure() {
            return None;
        }
        self.node.read.value.value.present().map(String::as_str)
    }

    pub(crate) fn child_count(&self) -> Option<usize> {
        let count = self.node.read.node.properties.child_count.present()?;
        usize::try_from(*count).ok()
    }

    pub(crate) fn secure(&self) -> bool {
        self.tokens()
            .is_some_and(|tokens| tokens.iter().any(|token| token == state::SECURE))
    }

    pub(crate) fn observation(&self) -> Observation {
        let value = if self.secure() {
            None
        } else {
            text_of(&self.node.read.value.value)
        };
        Observation {
            checked: self.state(CHECKED),
            expanded: self.state(EXPANDED),
            selected: self.state(SELECTED),
            pressed: self.state(PRESSED),
            value,
            name: text_of(&self.node.read.node.properties.name),
            child_count: self.child_count(),
        }
    }
}

/// What a write can change on a node, each part `None` when it was not read.
/// Focus, showing and any armed state never count: they move without the
/// write having done anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Observation {
    checked: Option<bool>,
    expanded: Option<bool>,
    selected: Option<bool>,
    pressed: Option<bool>,
    value: Option<Option<String>>,
    name: Option<Option<String>>,
    child_count: Option<usize>,
}

fn differs<T: PartialEq>(now: &Option<T>, before: &Option<T>) -> Option<bool> {
    Some(now.as_ref()? != before.as_ref()?)
}

impl Observation {
    /// Whether any part both readings share differs: `Some(true)` as soon as
    /// one does, `Some(false)` only when every part was read both times and
    /// none differs, `None` when an unread part leaves it undecided.
    pub(crate) fn changed_since(&self, before: &Observation) -> Option<bool> {
        let verdicts = [
            differs(&self.checked, &before.checked),
            differs(&self.expanded, &before.expanded),
            differs(&self.selected, &before.selected),
            differs(&self.pressed, &before.pressed),
            differs(&self.value, &before.value),
            differs(&self.name, &before.name),
            differs(&self.child_count, &before.child_count),
        ];
        if verdicts.contains(&Some(true)) {
            Some(true)
        } else if verdicts.contains(&None) {
            None
        } else {
            Some(false)
        }
    }
}

struct ViewReader<'a> {
    connection: &'a Connection,
    deadline: Deadline,
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

/// Reads the node a handle names once, with the answers a failed read gets
/// everywhere else: a destroyed object or an application that left is a
/// settled stale ref, a node that did not answer is a timeout.
pub(crate) fn read_node_view(
    bus: &SharedBus,
    handle: &NativeHandle,
    deadline: Deadline,
) -> Result<NodeView, AdapterError> {
    let connection = bus.get(deadline)?.inner().clone();
    let mut reader = ViewReader {
        connection: &connection,
        deadline,
    };
    read_live(&mut reader, handle).map(NodeView::new)
}

/// What one `GetRole` outcome says about the node. Every live node of every
/// toolkit answers it, so an error naming a missing object or application is
/// the second witness a destroyed node needs, a timeout proves nothing, and
/// any other failure is not a reading.
pub(crate) fn corroboration_of(role_error: Option<&zbus::Error>) -> Corroboration {
    match role_error.map(failure) {
        None => Corroboration::Alive,
        Some(Failure::AppGone | Failure::Suspect) => Corroboration::Dead,
        Some(Failure::TimedOut) => Corroboration::TimedOut,
        Some(Failure::Other) => Corroboration::NotRead,
    }
}

async fn role_outcome(connection: &Connection, element: &AtspiElement) -> Corroboration {
    let reply = call(connection, element, ACCESSIBLE_INTERFACE, "GetRole", &()).await;
    corroboration_of(reply.as_ref().err())
}

/// Asks the node whether it still answers, once, inside the deadline.
pub(crate) fn corroborate(
    bus: &SharedBus,
    element: &AtspiElement,
    deadline: Deadline,
) -> Corroboration {
    let Ok(session) = bus.get(deadline) else {
        return Corroboration::NotRead;
    };
    let connection = session.inner().clone();
    within(deadline, async {
        Ok(role_outcome(&connection, element).await)
    })
    .unwrap_or(Corroboration::TimedOut)
}

#[cfg(test)]
#[path = "action_read_tests.rs"]
mod tests;
