use super::menu_popup::{Node, only_menu_content};
use super::menu_surface::{PopupReader, popup_read_complete, vanished};
use crate::system::{REGISTRY_ROOT_PATH, SharedBus, within};
use crate::tree::observe_menu::{is_cut_short_beyond_pruning, rooted, scan_walk, scan_walk_pruned};
use crate::tree::role_numbers::{FRAME, WINDOW};
use crate::tree::state_bits::{SHOWING, StateBits};
use crate::tree::{AtspiElement, RawNode, list_children, read_node};
use agent_desktop_core::{AdapterError, Deadline};
use std::collections::HashMap;
use std::time::Duration;

const WINDOW_NODES: usize = 2000;
const TOP_LEVEL_NODES: usize = 80;

pub(in crate::system) struct Inventory<'a> {
    bus: &'a SharedBus,
    deadline: Deadline,
    window: Option<AtspiElement>,
    trees: HashMap<(String, String), Result<Node, AdapterError>>,
    top_levels: HashMap<String, (Vec<Node>, bool)>,
    incomplete: bool,
}

impl<'a> Inventory<'a> {
    pub(in crate::system) fn new(bus: &'a SharedBus, deadline: Deadline) -> Self {
        Self {
            bus,
            deadline,
            window: None,
            trees: HashMap::new(),
            top_levels: HashMap::new(),
            incomplete: false,
        }
    }

    pub(in crate::system) fn select(&mut self, window: &AtspiElement) {
        self.window = Some(window.clone());
        self.incomplete = false;
    }

    pub(in crate::system) fn read(&mut self, window: &AtspiElement) -> Result<Node, AdapterError> {
        let key = (window.bus_name().to_owned(), window.path().to_owned());
        self.trees
            .entry(key)
            .or_insert_with(|| scan_walk(self.bus, &rooted(window), WINDOW_NODES, self.deadline))
            .clone()
    }

    fn top(&mut self, window: &AtspiElement) -> Result<(Vec<Node>, bool), AdapterError> {
        let key = window.bus_name().to_owned();
        if let Some(cached) = self.top_levels.get(&key) {
            return Ok(cached.clone());
        }
        let connection = self.bus.get(self.deadline)?;
        let root = AtspiElement::new(window.bus_name(), REGISTRY_ROOT_PATH, window.pid());
        let children = within(self.deadline, list_children(connection.inner(), &root))?;
        let mut found = Vec::new();
        let mut incomplete = false;
        for (bus_name, path) in children {
            let child = AtspiElement::new(bus_name, path, window.pid());
            let key = (child.bus_name().to_owned(), child.path().to_owned());
            if let Some(Ok(tree)) = self.trees.get(&key) {
                if !is_cut_short_beyond_pruning(tree) {
                    incomplete |= !popup_read_complete(tree);
                    if is_popup_candidate(&tree.payload.node) {
                        found.push(tree.clone());
                    }
                    continue;
                }
            }
            let node = match within(self.deadline, read_node(connection.inner(), &child)) {
                Ok(node) => node,
                Err(error) => {
                    incomplete |= !vanished(&error);
                    continue;
                }
            };
            if node.traits.role.present().is_none()
                || node.traits.states.present().is_none()
                || node.properties.name.is_failed()
            {
                incomplete = true;
                continue;
            }
            if !is_popup_candidate(&node) {
                continue;
            }
            match scan_walk_pruned(self.bus, &rooted(&child), TOP_LEVEL_NODES, self.deadline) {
                Ok(tree) => {
                    incomplete |= may_hide_a_menu(&tree) || !popup_read_complete(&tree);
                    found.push(tree);
                }
                Err(error) => incomplete |= !vanished(&error),
            }
        }
        self.top_levels
            .insert(window.bus_name().to_owned(), (found.clone(), incomplete));
        Ok((found, incomplete))
    }
}

fn is_popup_candidate(node: &RawNode) -> bool {
    let shaped = matches!(node.traits.role.present(), Some(&(WINDOW | FRAME)));
    let showing = node
        .traits
        .states
        .present()
        .is_some_and(|words| StateBits::new(words).has(SHOWING));
    shaped && showing && node.properties.name.is_blank()
}

/// Whether a top-level candidate was read only in part and what was read is
/// all menu content: an open menu may continue past where the read stopped.
/// A cut-short window holding anything else is not a popup whatever it hides.
fn may_hide_a_menu(candidate: &Node) -> bool {
    is_cut_short_beyond_pruning(candidate) && candidate.children.iter().all(only_menu_content)
}

impl PopupReader for Inventory<'_> {
    fn incomplete(&self) -> bool {
        self.incomplete
    }

    fn window(&mut self) -> Result<Option<Node>, AdapterError> {
        let window = self
            .window
            .as_ref()
            .ok_or_else(|| AdapterError::internal("No inventory window selected"))?;
        let key = (window.bus_name().to_owned(), window.path().to_owned());
        if let Some(Ok(tree)) = self.trees.get(&key) {
            if !crate::tree::nested_dialog::is_incomplete(tree) {
                self.incomplete |= !popup_read_complete(tree);
                return Ok(Some(tree.clone()));
            }
        }
        let web = within(
            self.deadline,
            crate::tree::toolkit_trust::read_web_family(
                self.bus.get(self.deadline)?.inner(),
                window,
            ),
        )?
        .is_some();
        if web {
            return Ok(None);
        }
        let tree = scan_walk_pruned(self.bus, &rooted(window), WINDOW_NODES, self.deadline)?;
        self.incomplete |= is_cut_short_beyond_pruning(&tree) || !popup_read_complete(&tree);
        Ok(Some(tree))
    }

    fn top_levels(&mut self) -> Result<Vec<Node>, AdapterError> {
        let window = self
            .window
            .clone()
            .ok_or_else(|| AdapterError::internal("No inventory window selected"))?;
        let (all, incomplete) = self.top(&window)?;
        self.incomplete |= incomplete;
        Ok(all
            .into_iter()
            .filter(|node| {
                let element = &node.payload.node.element;
                !(element.bus_name() == window.bus_name() && element.path() == window.path())
            })
            .collect())
    }

    fn pause(&mut self, gap: Duration) {
        self.trees.clear();
        self.top_levels.clear();
        std::thread::sleep(gap);
    }
}

#[cfg(test)]
#[path = "inventory_reader_tests.rs"]
mod tests;
