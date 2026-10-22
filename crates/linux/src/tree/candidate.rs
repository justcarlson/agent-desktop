use agent_desktop_core::{BoundsSpace, Rect};

use super::AtspiElement;

/// A live node that answers to the stored evidence, with its extents for the
/// geometry tie-break and its depth below the window frame (the frame is 0, so a
/// node's depth is the length of its child-index path).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Candidate {
    pub(crate) element: AtspiElement,
    pub(crate) bounds: Option<Rect>,
    pub(crate) bounds_space: BoundsSpace,
    pub(crate) depth: usize,
}
