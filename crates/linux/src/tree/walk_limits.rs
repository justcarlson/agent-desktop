const DEFAULT_MAX_DEPTH: usize = 50;
const DEFAULT_MAX_NODES: usize = 10_000;

/// How far a walk may go before it marks the rest truncated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WalkLimits {
    pub(crate) max_depth: usize,
    pub(crate) max_nodes: usize,
}

impl Default for WalkLimits {
    fn default() -> Self {
        Self {
            max_depth: DEFAULT_MAX_DEPTH,
            max_nodes: DEFAULT_MAX_NODES,
        }
    }
}
