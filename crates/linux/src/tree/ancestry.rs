use super::AtspiElement;
use super::role_numbers::{SCROLL_PANE, VIEWPORT, is_web_document};

/// What a node needs after the walk, so later code never climbs the `Parent`
/// property — which the ledger records disagreeing with the walked parent
/// (L2-2): the window root, nearest scroll container, direct button parent,
/// and whether a web document encloses it. Ancestors are bare addresses,
/// never a chain.
#[derive(Debug, Clone, Default)]
pub(crate) struct Ancestry {
    frame: Option<Box<AtspiElement>>,
    scroll_container: Option<Box<AtspiElement>>,
    in_web_document: bool,
    button_parent: Option<Box<AtspiElement>>,
}

impl Ancestry {
    /// An ancestry from the two addresses and the web flag, each address kept
    /// without its own ancestry.
    pub(crate) fn of(
        frame: Option<AtspiElement>,
        scroll_container: Option<AtspiElement>,
        in_web_document: bool,
    ) -> Self {
        Self {
            frame: frame.map(|element| Box::new(element.address())),
            scroll_container: scroll_container.map(|element| Box::new(element.address())),
            in_web_document,
            button_parent: None,
        }
    }

    pub(crate) fn button_parent(&self) -> Option<AtspiElement> {
        let mut parent = self.button_parent.as_deref()?.clone();
        parent.set_ancestry(Self::of(
            self.frame.as_deref().cloned(),
            None,
            self.in_web_document,
        ));
        Some(parent)
    }

    pub(crate) fn frame(&self) -> Option<&AtspiElement> {
        self.frame.as_deref()
    }

    pub(crate) fn scroll_container(&self) -> Option<&AtspiElement> {
        self.scroll_container.as_deref()
    }

    pub(crate) fn in_web_document(&self) -> bool {
        self.in_web_document
    }
}

/// The ancestry of the walk's own root: a window root is its own frame and
/// sits above any web document.
pub(crate) fn of_root(element: &AtspiElement) -> Ancestry {
    Ancestry::of(Some(element.clone()), None, false)
}

/// The ancestry the node beneath `parent` records, from the parent's numeric
/// role: the frame is inherited, a scroll pane or a viewport parent is itself
/// the nearest container, and a web document parent, or a parent already
/// inside one, puts the child inside a web document.
pub(crate) fn child(parent: &AtspiElement, role: Option<u32>) -> Ancestry {
    let mut ancestry = Ancestry::of(
        parent.frame().cloned(),
        match role {
            Some(SCROLL_PANE | VIEWPORT) => Some(parent.address()),
            _ => parent.scroll_container().cloned(),
        },
        parent.in_web_document() || is_web_document(role),
    );
    if matches!(
        role,
        Some(super::role_numbers::PUSH_BUTTON | super::role_numbers::PUSH_BUTTON_MENU)
    ) {
        ancestry.button_parent = Some(Box::new(parent.address()));
    }
    ancestry
}

#[cfg(test)]
#[path = "ancestry_tests.rs"]
mod tests;
