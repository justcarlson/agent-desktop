use std::time::Duration;

use super::role_numbers::DOCUMENT_WEB;
use super::{ObservedRead, WalkedNode};

/// Gap between two full reads of a filling tree.
pub(crate) const POLL: Duration = Duration::from_millis(200);
const EQUAL_READS: usize = 3;
const SHELL_CEILING: usize = 32;
const BUDGET_PERCENT: u32 = 70;

/// What to do after a walk of a tree that may still be filling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    Settled,
    Wait(Duration),
    GiveUp,
}

/// Decides when a web tree has stopped growing. A tree is settled when three
/// consecutive full reads count the same nodes and the count is above the
/// size of the empty browser shell; the wait never spends more than a share
/// of the budget the observation started with.
#[derive(Debug, Clone)]
pub(crate) struct Settle {
    budget: Duration,
    last: Option<usize>,
    equal: usize,
}

impl Settle {
    /// `remaining` is what the request's deadline had left when the wait began.
    pub(crate) fn new(remaining: Duration) -> Self {
        Self {
            budget: remaining * BUDGET_PERCENT / 100,
            last: None,
            equal: 0,
        }
    }

    /// Takes the node count of the walk just finished and the time spent
    /// waiting so far.
    pub(crate) fn after_walk(&mut self, nodes: usize, waited: Duration) -> Step {
        self.equal = if self.last == Some(nodes) {
            self.equal + 1
        } else {
            1
        };
        self.last = Some(nodes);
        if self.equal >= EQUAL_READS && nodes > SHELL_CEILING {
            Step::Settled
        } else if waited + POLL > self.budget {
            Step::GiveUp
        } else {
            Step::Wait(POLL)
        }
    }
}

/// Whether the walked tree contains a web document, which marks a web
/// process whose binary name gave no hint.
pub(crate) fn has_web_document(root: &WalkedNode<ObservedRead>) -> bool {
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if node.payload.node.traits.role.present() == Some(&DOCUMENT_WEB) {
            return true;
        }
        pending.extend(&node.children);
    }
    false
}

#[cfg(test)]
#[path = "settle_tests.rs"]
mod tests;
