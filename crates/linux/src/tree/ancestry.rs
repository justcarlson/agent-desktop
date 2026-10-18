use super::AtspiElement;
use super::role_numbers::{SCROLL_PANE, VIEWPORT};

/// The ancestors a node needs after the walk, so later code never climbs the
/// `Parent` property — which the ledger records disagreeing with the walked
/// parent (L2-2): the window root the node belongs to and the nearest scroll
/// container it sits inside. Each is kept as a bare address, never a chain.
#[derive(Debug, Clone, Default)]
pub(crate) struct Ancestry {
    frame: Option<Box<AtspiElement>>,
    scroll_container: Option<Box<AtspiElement>>,
}

impl Ancestry {
    /// An ancestry from the two addresses, each kept without its own.
    pub(crate) fn of(frame: Option<AtspiElement>, scroll_container: Option<AtspiElement>) -> Self {
        Self {
            frame: frame.map(|element| Box::new(element.address())),
            scroll_container: scroll_container.map(|element| Box::new(element.address())),
        }
    }

    pub(crate) fn frame(&self) -> Option<&AtspiElement> {
        self.frame.as_deref()
    }

    pub(crate) fn scroll_container(&self) -> Option<&AtspiElement> {
        self.scroll_container.as_deref()
    }
}

/// The ancestry of the walk's own root: a window root is its own frame.
pub(crate) fn of_root(element: &AtspiElement) -> Ancestry {
    Ancestry::of(Some(element.clone()), None)
}

/// The ancestry the node beneath `parent` records, from the parent's numeric
/// role: the frame is inherited, and a scroll pane or a viewport parent is
/// itself the nearest container.
pub(crate) fn child(parent: &AtspiElement, role: Option<u32>) -> Ancestry {
    Ancestry::of(
        parent.frame().cloned(),
        match role {
            Some(SCROLL_PANE | VIEWPORT) => Some(parent.address()),
            _ => parent.scroll_container().cloned(),
        },
    )
}

/// Picks the ancestry a node keeps from the chain of its ancestors, root
/// first: the frame is the chain's first element and the nearest scroll pane
/// or viewport wins when several enclose it. The resolver's path descent is
/// the caller: the walk derives ancestry incrementally through `child`.
#[allow(dead_code)]
pub(crate) fn select(chain: &[(AtspiElement, Option<u32>)]) -> Ancestry {
    Ancestry::of(
        chain.first().map(|(element, _)| element.clone()),
        chain
            .iter()
            .rev()
            .find(|(_, role)| matches!(role, Some(SCROLL_PANE | VIEWPORT)))
            .map(|(element, _)| element.clone()),
    )
}

#[cfg(test)]
#[path = "ancestry_tests.rs"]
mod tests;
