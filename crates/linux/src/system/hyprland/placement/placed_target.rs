use agent_desktop_core::Point;

/// The bound toplevel a ref element belongs to and the global layout point
/// the pointer aims at inside it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlacedTarget {
    pub(crate) address: u64,
    pub(crate) at: Point,
}
