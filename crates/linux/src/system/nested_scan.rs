use agent_desktop_core::{AdapterError, Deadline};

use super::listed_window::ListedWindow;
use super::{SharedBus, window_id};
use crate::tree::AtspiElement;
use crate::tree::nested_dialog::{Node, find, is_cut_short};
use crate::tree::observe_menu::{rooted, scan_walk};

const NESTED_NODES: usize = 1000;

/// A dialog drawn inside the tree of the window at `window`: the id is the
/// dialog's own address, which no window shares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Nested {
    pub(crate) window: usize,
    pub(crate) id: String,
    pub(crate) title: Option<String>,
}

/// What a scan of the windows of one application found, and whether it read
/// every window to the end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NestedScan {
    pub(crate) found: Vec<Nested>,
    pub(crate) complete: bool,
}

impl NestedScan {
    pub(crate) fn complete_and_empty() -> Self {
        Self {
            found: Vec::new(),
            complete: true,
        }
    }
}

/// Reads one window's tree to a bounded size.
pub(crate) fn bus_read<'a>(
    bus: &'a SharedBus,
    deadline: Deadline,
) -> impl Fn(&AtspiElement) -> Result<Node, AdapterError> + 'a {
    move |window| scan_walk(bus, &rooted(window), NESTED_NODES, deadline)
}

/// The showing dialogs inside the windows, other than windows that are
/// dialogs themselves. A window that cannot be read, or whose tree was cut
/// short, makes the scan incomplete: a dialog may sit where it did not look.
pub(crate) fn scan_windows(
    windows: &[ListedWindow],
    read: impl Fn(&AtspiElement) -> Result<Node, AdapterError>,
) -> NestedScan {
    let mut scan = NestedScan::complete_and_empty();
    for (index, window) in windows.iter().enumerate().filter(|(_, w)| !w.sheet) {
        let Ok((bus_name, path)) = window_id::decode(&window.info.id) else {
            scan.complete = false;
            continue;
        };
        let element = AtspiElement::new(bus_name, path, window.info.pid.get());
        let Ok(root) = read(&element) else {
            scan.complete = false;
            continue;
        };
        scan.complete &= !is_cut_short(&root);
        for dialog in find(&root) {
            match window_id::encode(dialog.element.bus_name(), dialog.element.path()) {
                Some(id) => scan.found.push(Nested {
                    window: index,
                    id,
                    title: dialog.title.map(str::to_owned),
                }),
                None => scan.complete = false,
            }
        }
    }
    scan
}

#[cfg(test)]
#[path = "nested_scan_tests.rs"]
mod tests;
