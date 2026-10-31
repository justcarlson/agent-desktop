use serde::{Deserialize, Serialize};

/// The observers that each reported the focused window active after
/// `focus-window`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FocusProof {
    /// The compositor and the window's own accessibility frame.
    CompositorAndAccessibility,
    /// The compositor only, because the window exposes no accessibility tree.
    CompositorOnly,
}
