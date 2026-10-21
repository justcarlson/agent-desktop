use super::{WalkCounts, WalkedNode};

/// A finished walk. `complete` is false when any subtree was cut short.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Walk<P> {
    pub(crate) root: WalkedNode<P>,
    pub(crate) complete: bool,
    pub(crate) node_count: usize,
    pub(crate) max_depth: usize,
    pub(crate) counts: WalkCounts,
}
