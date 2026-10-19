use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline};

use super::menu_popup::Node;
use super::menu_surface::PopupReader;
use super::{REGISTRY_ROOT_PATH, SharedBus, process_facts, within};
use crate::tree::observe_menu::{rooted, scan_walk};
use crate::tree::role_numbers::{FRAME, WINDOW};
use crate::tree::state_bits::{SHOWING, StateBits};
use crate::tree::{AtspiElement, RawNode, list_children, read_node};

const WINDOW_NODES: usize = 2000;
const TOP_LEVEL_NODES: usize = 80;

/// Reads the popups of one window from the accessibility bus.
pub(super) struct BusReader<'a> {
    bus: &'a SharedBus,
    window: &'a AtspiElement,
    deadline: Deadline,
}

impl<'a> BusReader<'a> {
    pub(super) fn new(bus: &'a SharedBus, window: &'a AtspiElement, deadline: Deadline) -> Self {
        Self {
            bus,
            window,
            deadline,
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

impl PopupReader for BusReader<'_> {
    fn window(&mut self) -> Result<Option<Node>, AdapterError> {
        let web =
            process_facts::read(self.window.pid()).is_some_and(|facts| facts.web_family.is_some());
        if web {
            return Ok(None);
        }
        scan_walk(self.bus, &rooted(self.window), WINDOW_NODES, self.deadline).map(Some)
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
            let Ok(node) = within(self.deadline, read_node(connection.inner(), &child)) else {
                continue;
            };
            if !is_popup_candidate(&node) {
                continue;
            }
            if let Ok(walked) = scan_walk(self.bus, &rooted(&child), TOP_LEVEL_NODES, self.deadline)
            {
                windows.push(walked);
            }
        }
        Ok(windows)
    }

    fn pause(&mut self, gap: Duration) {
        std::thread::sleep(gap);
    }
}
