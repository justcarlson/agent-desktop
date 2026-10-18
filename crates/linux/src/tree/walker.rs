use agent_desktop_core::{AdapterError, Deadline, ErrorCode};

use super::walker_expand::expand_level;
use super::{AtspiElement, RawNode, Walk, WalkLimits, WalkedNode};
use crate::system::SharedBus;

struct Pending<K> {
    key: K,
    parent: Option<usize>,
    depth: usize,
}

struct Slot<K, P> {
    key: K,
    payload: P,
    parent: Option<usize>,
    children: Vec<usize>,
    truncated: bool,
    depth: usize,
}

/// Decides which of a node's children the walk may descend into. Returns the
/// indices admitted, in order, and whether any child was left out by a limit.
/// A child already on the path from the root is skipped as a cycle; that is
/// not a truncation. A child shared by two siblings' subtrees is not on the
/// path and is admitted under both.
pub(crate) fn admit_children<K: PartialEq>(
    children: &[K],
    ancestors: &[&K],
    depth: usize,
    admitted: usize,
    limits: &WalkLimits,
) -> (Vec<usize>, bool) {
    if children.is_empty() {
        return (Vec::new(), false);
    }
    if depth >= limits.max_depth {
        return (Vec::new(), true);
    }
    let room = limits.max_nodes.saturating_sub(admitted);
    let eligible: Vec<usize> = (0..children.len())
        .filter(|index| !ancestors.contains(&&children[*index]))
        .collect();
    let truncated = eligible.len() > room;
    (eligible.into_iter().take(room).collect(), truncated)
}

fn ancestor_path<K, P>(slots: &[Slot<K, P>], index: usize) -> Vec<&K> {
    let mut path = Vec::new();
    let mut cursor = Some(index);
    while let Some(current) = cursor {
        path.push(&slots[current].key);
        cursor = slots[current].parent;
    }
    path
}

fn assemble<K, P>(slots: Vec<Slot<K, P>>) -> Option<WalkedNode<P>> {
    let mut built: Vec<Option<WalkedNode<P>>> = (0..slots.len()).map(|_| None).collect();
    for (index, slot) in slots.into_iter().enumerate().rev() {
        let children = slot
            .children
            .iter()
            .filter_map(|child| built[*child].take())
            .collect();
        built[index] = Some(WalkedNode {
            payload: slot.payload,
            children,
            truncated: slot.truncated,
        });
    }
    built.first_mut().and_then(Option::take)
}

/// Breadth-first walk. `expand` reads a whole level at once and returns, for
/// each key in order, its payload and its children. Stops descending when
/// the deadline has passed, marking the parents of the unread level.
pub(crate) fn walk_levels<K, P>(
    root: K,
    limits: &WalkLimits,
    deadline: Deadline,
    mut expand: impl FnMut(&[K]) -> Result<Vec<(P, Vec<K>)>, AdapterError>,
) -> Result<Walk<P>, AdapterError>
where
    K: Clone + PartialEq,
{
    let mut slots: Vec<Slot<K, P>> = Vec::new();
    let mut frontier = vec![Pending {
        key: root,
        parent: None,
        depth: 0,
    }];
    let mut admitted = 1;
    let mut complete = true;
    while !frontier.is_empty() {
        if deadline.is_expired() {
            for pending in &frontier {
                if let Some(parent) = pending.parent {
                    slots[parent].truncated = true;
                }
            }
            complete = false;
            break;
        }
        let keys: Vec<K> = frontier.iter().map(|pending| pending.key.clone()).collect();
        let reads = match expand(&keys) {
            Ok(reads) => reads,
            Err(error) if error.code == ErrorCode::Timeout && !slots.is_empty() => {
                for pending in &frontier {
                    if let Some(parent) = pending.parent {
                        slots[parent].truncated = true;
                    }
                }
                complete = false;
                break;
            }
            Err(error) => return Err(error),
        };
        let mut next = Vec::new();
        for (pending, (payload, children)) in frontier.into_iter().zip(reads) {
            let index = slots.len();
            if let Some(parent) = pending.parent {
                slots[parent].children.push(index);
            }
            slots.push(Slot {
                key: pending.key,
                payload,
                parent: pending.parent,
                children: Vec::new(),
                truncated: false,
                depth: pending.depth,
            });
            let (taken, truncated) = {
                let path = ancestor_path(&slots, index);
                admit_children(&children, &path, pending.depth, admitted, limits)
            };
            admitted += taken.len();
            if truncated {
                slots[index].truncated = true;
                complete = false;
            }
            next.extend(taken.into_iter().map(|child| Pending {
                key: children[child].clone(),
                parent: Some(index),
                depth: pending.depth + 1,
            }));
        }
        frontier = next;
    }
    let node_count = slots.len();
    let max_depth = slots.iter().map(|slot| slot.depth).max().unwrap_or(0);
    let root = assemble(slots).ok_or_else(|| {
        AdapterError::new(ErrorCode::Internal, "The accessibility walk read no nodes")
    })?;
    Ok(Walk {
        root,
        complete,
        node_count,
        max_depth,
    })
}

/// Walks the tree under `root` on the shared accessibility connection.
pub(crate) fn walk_tree(
    bus: &SharedBus,
    root: &AtspiElement,
    limits: &WalkLimits,
    deadline: Deadline,
) -> Result<Walk<RawNode>, AdapterError> {
    let connection = bus.get(deadline)?;
    let mut pids = std::collections::HashMap::from([(root.bus_name().to_owned(), root.pid())]);
    let mut at_root = true;
    walk_levels(root.clone(), limits, deadline, |level| {
        let reads = expand_level(connection.inner(), &mut pids, level, at_root, deadline);
        at_root = false;
        reads
    })
}
