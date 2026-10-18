use agent_desktop_core::LocatorLimitStats;

use super::WalkLimits;

/// Which of a node's children the walk may descend into.
pub(super) struct Admission {
    pub(super) indices: Vec<usize>,
    pub(super) truncated: bool,
    pub(super) considered: usize,
    pub(super) cycles: u64,
    pub(super) hits: LocatorLimitStats,
}

/// Decides which children of a node are admitted. `indices` are positions in
/// the node's own child list, in order. A child already on the path from the
/// root is skipped as a cycle; that is not a truncation. A child shared by
/// two siblings' subtrees is not on the path and is admitted under both. A
/// child list longer than the per-node cap, an edge budget that is spent and a
/// node budget that is spent each cut the list and are counted.
pub(super) fn admit_children<K: PartialEq>(
    children: &[K],
    ancestors: &[&K],
    admitted: usize,
    edges: usize,
    limits: &WalkLimits,
) -> Admission {
    let mut hits = LocatorLimitStats::default();
    let mut considered = children.len();
    if considered > limits.max_children {
        considered = limits.max_children;
        hits.child_hits = 1;
    }
    let edge_room = limits.max_edges.saturating_sub(edges);
    if considered > edge_room {
        considered = edge_room;
        hits.edge_hits = 1;
    }
    let mut cycles = 0;
    let mut eligible = Vec::with_capacity(considered);
    for (index, child) in children.iter().enumerate().take(considered) {
        if ancestors.contains(&child) {
            cycles += 1;
        } else {
            eligible.push(index);
        }
    }
    let room = limits.max_nodes.saturating_sub(admitted);
    if eligible.len() > room {
        hits.node_hits = 1;
    }
    let truncated = hits.child_hits + hits.edge_hits + hits.node_hits > 0;
    eligible.truncate(room);
    Admission {
        indices: eligible,
        truncated,
        considered,
        cycles,
        hits,
    }
}
