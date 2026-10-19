use super::Extents;

/// The widest a window rectangle may overhang the frame and still count as
/// inside it, in pixels.
#[expect(dead_code, reason = "stub the owning lane replaces")]
pub(crate) const EDGE_SLACK: i32 = 24;

/// Everything the offscreen rule reads about one node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OffscreenInput {
    pub(crate) node: Option<Extents>,
    pub(crate) frame: Option<Extents>,
    pub(crate) container: Option<Extents>,
    pub(crate) visible: bool,
    pub(crate) showing: bool,
    pub(crate) sentinel: bool,
    pub(crate) minimized: Option<bool>,
}

/// `Some(true)` only when the node is provably off screen, `Some(false)` when
/// it provably intersects what shows it, `None` otherwise.
#[expect(dead_code, reason = "stub the owning lane replaces")]
pub(crate) fn offscreen(_input: OffscreenInput) -> Option<bool> {
    None
}
