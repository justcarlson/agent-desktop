use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline};

use super::menu_popup::{Node, only_menu_content};
use super::menu_surface::{PopupReader, vanished};
use super::{REGISTRY_ROOT_PATH, SharedBus, process_facts, within};
use crate::tree::observe_menu::{is_cut_short_beyond_pruning, rooted, scan_walk_pruned};
use crate::tree::role_numbers::{FRAME, WINDOW};
use crate::tree::state_bits::{SHOWING, StateBits};
use crate::tree::{AtspiElement, RawNode, list_children, read_node};

const WINDOW_NODES: usize = 2000;
const TOP_LEVEL_NODES: usize = 80;

/// Reads the popups of one window from the accessibility bus.
pub(super) struct BusReader<'a> {
    pub(super) bus: &'a SharedBus,
    pub(super) window: &'a AtspiElement,
    pub(super) deadline: Deadline,
    /// Set when a read failed or was cut short in a place a popup could be
    /// hiding, so that an empty answer is not a closed one.
    pub(super) incomplete: bool,
}

impl<'a> BusReader<'a> {
    pub(super) fn new(bus: &'a SharedBus, window: &'a AtspiElement, deadline: Deadline) -> Self {
        Self {
            bus,
            window,
            deadline,
            incomplete: false,
        }
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

impl PopupReader for BusReader<'_> {
    fn incomplete(&self) -> bool {
        self.incomplete
    }

    fn window(&mut self) -> Result<Option<Node>, AdapterError> {
        let web =
            process_facts::read(self.window.pid()).is_some_and(|facts| facts.web_family.is_some());
        if web {
            return Ok(None);
        }
        let walked = scan_walk_pruned(self.bus, &rooted(self.window), WINDOW_NODES, self.deadline)?;
        self.incomplete |= is_cut_short_beyond_pruning(&walked);
        Ok(Some(walked))
    }

    fn top_levels(&mut self) -> Result<Vec<Node>, AdapterError> {
        let connection = self.bus.get(self.deadline)?;
        let pid = self.window.pid();
        let root = AtspiElement::new(self.window.bus_name(), REGISTRY_ROOT_PATH, pid);
        let children = within(self.deadline, list_children(connection.inner(), &root))?;
        let mut windows = Vec::new();
        for (bus_name, path) in children {
            if bus_name == self.window.bus_name() && path == self.window.path() {
                continue;
            }
            let child = AtspiElement::new(bus_name, path, pid);
            let node = match within(self.deadline, read_node(connection.inner(), &child)) {
                Ok(node) => node,
                Err(error) => {
                    self.incomplete |= !vanished(&error);
                    continue;
                }
            };
            if !is_popup_candidate(&node) {
                continue;
            }
            match scan_walk_pruned(self.bus, &rooted(&child), TOP_LEVEL_NODES, self.deadline) {
                Ok(walked) => {
                    self.incomplete |= may_hide_a_menu(&walked);
                    windows.push(walked);
                }
                Err(error) => self.incomplete |= !vanished(&error),
            }
        }
        Ok(windows)
    }

    fn pause(&mut self, gap: Duration) {
        std::thread::sleep(gap);
    }
}

#[cfg(test)]
#[path = "menu_reader_tests.rs"]
mod tests;
