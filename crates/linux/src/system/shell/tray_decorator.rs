use agent_desktop_core::{LocatorEvidence, LocatorField, ObservedSubtree, capability};

use super::container::MENU;
use crate::tray::view::TrayView;
use crate::tree::decorate::Decorate;
use crate::tree::{ObservedRead, WalkedNode};

const SYSTEM_BUTTON: &str = "System";

/// What the decorator needs from a tray read, so its rules run against
/// fixed items in a test.
pub(crate) trait Items {
    fn bind(&self, panel_titles: &[String]) -> Vec<Option<usize>>;
    fn decorate(
        &self,
        item: usize,
        evidence: &mut LocatorEvidence,
        first_child_index: usize,
    ) -> Vec<ObservedSubtree>;
    fn mark_unbound(&self, evidence: &mut LocatorEvidence);
    fn unpaired(&self, bound: &[Option<usize>]) -> Vec<ObservedSubtree>;
}

impl Items for TrayView {
    fn bind(&self, panel_titles: &[String]) -> Vec<Option<usize>> {
        TrayView::bind(self, panel_titles)
    }

    fn decorate(
        &self,
        item: usize,
        evidence: &mut LocatorEvidence,
        first_child_index: usize,
    ) -> Vec<ObservedSubtree> {
        TrayView::decorate(self, item, evidence, first_child_index)
    }

    fn mark_unbound(&self, evidence: &mut LocatorEvidence) {
        TrayView::mark_unbound(evidence);
    }

    fn unpaired(&self, bound: &[Option<usize>]) -> Vec<ObservedSubtree> {
        TrayView::unpaired(self, bound)
    }
}

/// The title of a panel node that may show a tray item: a named menu node
/// other than the system menu, which belongs to the quick settings.
fn panel_title(read: &ObservedRead) -> Option<String> {
    let is_menu = read.node.traits.role.present() == Some(&MENU);
    let name = read.node.properties.name.non_blank()?;
    (is_menu && name != SYSTEM_BUTTON).then(|| name.clone())
}

fn is_system_button(read: &ObservedRead) -> bool {
    read.node.traits.role.present() == Some(&MENU)
        && read.node.properties.name.non_blank().map(String::as_str) == Some(SYSTEM_BUTTON)
}

/// The system button advertises no accessibility action: the shell opens it
/// on a pointer press. It is addressable for a click all the same, so the
/// click capability is given to it here, for the physical delivery to carry.
fn pointer_pressable(evidence: &mut LocatorEvidence) {
    let has_none = evidence
        .ref_evidence
        .available_actions
        .known()
        .is_none_or(Vec::is_empty);
    if has_none {
        evidence.ref_evidence.available_actions =
            LocatorField::Known(vec![capability::CLICK.to_owned()]);
    }
}

fn titles(node: &WalkedNode<ObservedRead>, out: &mut Vec<String>) {
    out.extend(panel_title(&node.payload));
    for child in &node.children {
        titles(child, out);
    }
}

/// Attaches the tray's D-Bus identity to the panel nodes it can be paired
/// with, marks the rest unbound, and appends the items no node showed. With
/// no tray read it changes nothing: the tray is evidence, not a precondition.
pub(crate) struct TrayDecorator<I: Items> {
    items: Option<I>,
    bound: Vec<Option<usize>>,
    cursor: usize,
}

impl<I: Items> TrayDecorator<I> {
    pub(crate) fn new(items: Option<I>) -> Self {
        Self {
            items,
            bound: Vec::new(),
            cursor: 0,
        }
    }
}

impl<I: Items> Decorate for TrayDecorator<I> {
    fn prepare(&mut self, root: &WalkedNode<ObservedRead>) {
        let Some(items) = &self.items else {
            return;
        };
        let mut found = Vec::new();
        titles(root, &mut found);
        self.bound = items.bind(&found);
        self.cursor = 0;
    }

    fn decorate(
        &mut self,
        read: &ObservedRead,
        evidence: &mut LocatorEvidence,
        first_child_index: usize,
    ) -> Vec<ObservedSubtree> {
        if is_system_button(read) {
            pointer_pressable(evidence);
            return Vec::new();
        }
        let Some(items) = &self.items else {
            return Vec::new();
        };
        if panel_title(read).is_none() {
            return Vec::new();
        }
        let slot = self.bound.get(self.cursor).copied().flatten();
        self.cursor += 1;
        match slot {
            Some(item) => items.decorate(item, evidence, first_child_index),
            None => {
                items.mark_unbound(evidence);
                Vec::new()
            }
        }
    }

    fn finish(&mut self, first_child_index: usize) -> Vec<ObservedSubtree> {
        let Some(items) = &self.items else {
            return Vec::new();
        };
        items
            .unpaired(&self.bound)
            .into_iter()
            .enumerate()
            .map(|(offset, node)| node.with_source_child_index(first_child_index + offset))
            .collect()
    }
}

#[cfg(test)]
#[path = "tray_decorator_tests.rs"]
mod tests;
