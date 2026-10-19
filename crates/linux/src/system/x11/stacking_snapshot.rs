use super::stacked_window::StackedWindow;
use crate::tree::Extents;

/// The stacking order at one instant: managed clients bottom to top, the
/// current desktop, and the mapped override-redirect windows above the target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StackingSnapshot {
    pub(crate) current_desktop: Option<u32>,
    pub(crate) stack: Vec<StackedWindow>,
    pub(crate) override_redirect_above: Vec<Extents>,
}
