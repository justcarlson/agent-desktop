use agent_desktop_core::{AdapterError, Deadline};

use super::listed_window::ListedWindow;
use super::menu_popup;
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
    /// The menus the windows hold or are, keyed on the window's own id for a
    /// window that is itself a menu and on the popup's address for a menu
    /// drawn inside a window.
    pub(crate) menus: Vec<Nested>,
    pub(crate) complete: bool,
    /// The windows whose trees were not read to the end, by position.
    pub(crate) unread: Vec<usize>,
}

impl NestedScan {
    pub(crate) fn complete_and_empty() -> Self {
        Self {
            found: Vec::new(),
            menus: Vec::new(),
            complete: true,
            unread: Vec::new(),
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
            scan.unread.push(index);
            continue;
        };
        let element = AtspiElement::new(bus_name, path, window.info.pid.get());
        let Ok(root) = read(&element) else {
            scan.unread.push(index);
            continue;
        };
        let mut whole = !is_cut_short(&root);
        for dialog in find(&root) {
            match window_id::encode(dialog.element.bus_name(), dialog.element.path()) {
                Some(id) => scan.found.push(Nested {
                    window: index,
                    id,
                    title: dialog.title.map(str::to_owned),
                }),
                None => whole = false,
            }
        }
        if menu_popup::top_level(&root).is_some() {
            scan.menus.push(Nested {
                window: index,
                id: window.info.id.clone(),
                title: None,
            });
        } else {
            for menu in menu_popup::in_window(&root) {
                let element = &menu.root.payload.node.element;
                match window_id::encode(element.bus_name(), element.path()) {
                    Some(id) => scan.menus.push(Nested {
                        window: index,
                        id,
                        title: None,
                    }),
                    None => whole = false,
                }
            }
        }
        if !whole {
            scan.unread.push(index);
        }
    }
    scan.complete = scan.unread.is_empty();
    scan
}

#[cfg(test)]
#[path = "nested_scan_tests.rs"]
mod tests;
