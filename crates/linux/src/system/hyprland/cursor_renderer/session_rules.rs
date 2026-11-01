//! What the renderer asks of its session's manifest: whether the session
//! still wants a cursor, and whether it is a multi-agent session, where
//! every cursor control must name its agent.

use super::live_launcher::{session_is_multi_agent, session_wants_cursor};

/// The session checks a renderer runs; tests replace them.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SessionRules {
    pub(crate) wanted: fn(&str) -> bool,
    pub(crate) multi_agent: fn(&str) -> bool,
}

impl SessionRules {
    /// The checks that read the session store.
    pub(crate) fn live() -> Self {
        Self {
            wanted: session_wants_cursor,
            multi_agent: session_is_multi_agent,
        }
    }
}
