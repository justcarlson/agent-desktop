use std::collections::HashMap;
use std::sync::{Mutex, OnceLock, PoisonError};

use agent_desktop_core::{AdapterError, Deadline, ErrorCode};
use serde_json::json;

use super::identity::ShellRoot;
use super::light::{Light, LiveNodes, Nodes};
use super::search::{Query, find};
use crate::system::SharedBus;
use crate::tree::AtspiElement;

pub(crate) const MENU: u32 = 33;
pub(crate) const PUSH_BUTTON: u32 = 43;
pub(crate) const TOGGLE_BUTTON: u32 = 62;
const PANEL: u32 = 39;

const OVERVIEW: &str = "Overview";
const CLEAR_ALL: &str = "Clear all notifications";
const POWER_MENU: &str = "Power Off Menu";
const SYSTEM_BUTTON: &str = "System";
const SHOW_APPS: &str = "Show Apps";
const BAR_MIN_WIDTH: i32 = 600;
const BAR_MAX_HEIGHT: i32 = 64;
const LIST_ASCENT: usize = 2;
const SYSTEM_POPUP_ASCENT: usize = 6;
const TRAY_ASCENT: usize = 2;
const DOCK_ASCENT: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Landmark {
    MessageList,
    TrayPanel,
    DockPanel,
    SystemMenu,
    DateMenu,
}

/// The node a search anchors on. A landmark's container is found a fixed
/// number of levels above its anchor, so a container that holds nothing
/// readable while closed is still found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Anchor {
    Landmark(Landmark),
    SystemButton,
}

/// An anchor found in the tree and the container it stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Located {
    pub(crate) anchor: Light,
    pub(crate) container: Light,
}

fn ascent(anchor: Anchor) -> usize {
    match anchor {
        Anchor::Landmark(Landmark::MessageList) => LIST_ASCENT,
        Anchor::Landmark(Landmark::TrayPanel) => TRAY_ASCENT,
        Anchor::Landmark(Landmark::SystemMenu) => SYSTEM_POPUP_ASCENT,
        Anchor::Landmark(Landmark::DockPanel) => DOCK_ASCENT,
        Anchor::Landmark(Landmark::DateMenu) | Anchor::SystemButton => 0,
    }
}

pub(crate) fn is_anchor(anchor: Anchor, light: &Light) -> bool {
    match anchor {
        Anchor::Landmark(Landmark::MessageList) => light.is(PUSH_BUTTON, CLEAR_ALL),
        Anchor::Landmark(Landmark::TrayPanel) | Anchor::SystemButton => {
            light.is(MENU, SYSTEM_BUTTON)
        }
        Anchor::Landmark(Landmark::SystemMenu) => light.is(PUSH_BUTTON, POWER_MENU),
        Anchor::Landmark(Landmark::DockPanel) => {
            light.is(TOGGLE_BUTTON, SHOW_APPS) && light.placed().is_some()
        }
        Anchor::Landmark(Landmark::DateMenu) => light.role == MENU && light.name.is_empty(),
    }
}

fn is_top_bar(light: &Light) -> bool {
    light.role == PANEL
        && light.placed().is_some_and(|extents| {
            extents.x == 0
                && extents.y == 0
                && extents.width >= BAR_MIN_WIDTH
                && extents.height <= BAR_MAX_HEIGHT
        })
}

fn centred_in_top_bar(above: &[Light], light: &Light) -> bool {
    let (Some(bar), Some(own)) = (
        above.iter().find(|ancestor| is_top_bar(ancestor)),
        light.placed(),
    ) else {
        return false;
    };
    let Some(bar) = bar.placed() else {
        return false;
    };
    let bar_centre = i64::from(bar.x) + i64::from(bar.width) / 2;
    let own_centre = i64::from(own.x) + i64::from(own.width) / 2;
    (bar_centre - own_centre).abs() <= i64::from(bar.width) / 4
}

fn accepts(anchor: Anchor, above: &[Light], light: &Light) -> bool {
    match anchor {
        Anchor::Landmark(Landmark::DateMenu) => centred_in_top_bar(above, light),
        _ => true,
    }
}

fn not_found(anchor: Anchor) -> AdapterError {
    let (what, suggestion) = match anchor {
        Anchor::Landmark(Landmark::MessageList | Landmark::DateMenu) => (
            "message list",
            "The shell does not show a message list; check that it is a GNOME Shell session",
        ),
        Anchor::Landmark(Landmark::SystemMenu) | Anchor::SystemButton => (
            "system menu",
            "The shell does not show a system menu; check that it is a GNOME Shell session",
        ),
        Anchor::Landmark(Landmark::TrayPanel) => (
            "top bar tray",
            "The shell does not show a top bar; check that it is a GNOME Shell session",
        ),
        Anchor::Landmark(Landmark::DockPanel) => (
            "dock",
            "No dock is shown; enable a dock extension to snapshot the taskbar surface",
        ),
    };
    AdapterError::new(
        ErrorCode::ElementNotFound,
        format!("The shell's {what} was not found in its accessibility tree"),
    )
    .with_suggestion(suggestion)
    .with_details(json!({ "landmark": format!("{anchor:?}") }))
}

fn overview(light: &Light) -> bool {
    light.role == PANEL && light.name == OVERVIEW
}

fn above_anchor(path: &[Light], levels: usize) -> Option<&Light> {
    path.len().checked_sub(levels + 1).and_then(|i| path.get(i))
}

/// Remembered finds, so a second lookup in one process costs two reads. An
/// entry is trusted only after the anchor still answers to its rule.
#[derive(Default)]
pub(crate) struct Remembered {
    found: Mutex<HashMap<Anchor, (String, AtspiElement, AtspiElement)>>,
}

impl Remembered {
    fn recall<N: Nodes>(&self, nodes: &N, root: &ShellRoot, anchor: Anchor) -> Option<Located> {
        let (bus_name, anchor_element, container_element) = self
            .found
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&anchor)
            .cloned()?;
        if bus_name != root.bus_name {
            return None;
        }
        let light = nodes.read(&anchor_element).ok()?;
        if !is_anchor(anchor, &light) {
            return None;
        }
        let container = nodes.read(&container_element).ok()?;
        Some(Located {
            anchor: light,
            container,
        })
    }

    fn keep(&self, root: &ShellRoot, anchor: Anchor, located: &Located) {
        self.found
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(
                anchor,
                (
                    root.bus_name.clone(),
                    located.anchor.element.clone(),
                    located.container.element.clone(),
                ),
            );
    }
}

pub(crate) fn locate_in<N: Nodes>(
    nodes: &N,
    remembered: &Remembered,
    root: &ShellRoot,
    anchor: Anchor,
    deadline: Deadline,
) -> Result<Located, AdapterError> {
    if let Some(located) = remembered.recall(nodes, root, anchor) {
        return Ok(located);
    }
    let start = AtspiElement::new(root.bus_name.as_str(), root.path.as_str(), root.pid);
    let query = Query {
        prune: &overview,
        candidate: &|light| is_anchor(anchor, light),
        accept: &|above, light| accepts(anchor, above, light),
    };
    let path = find(nodes, &start, deadline, &query)?.ok_or_else(|| not_found(anchor))?;
    let found = path.last().cloned().ok_or_else(|| not_found(anchor))?;
    let container = above_anchor(&path, ascent(anchor))
        .cloned()
        .ok_or_else(|| not_found(anchor))?;
    let located = Located {
        anchor: found,
        container,
    };
    remembered.keep(root, anchor, &located);
    Ok(located)
}

fn process_memory() -> &'static Remembered {
    static MEMORY: OnceLock<Remembered> = OnceLock::new();
    MEMORY.get_or_init(Remembered::default)
}

pub(crate) fn locate_anchor(
    bus: &SharedBus,
    root: &ShellRoot,
    anchor: Anchor,
    deadline: Deadline,
) -> Result<Located, AdapterError> {
    let connection = bus.get(deadline)?.inner().clone();
    let nodes = LiveNodes::new(connection, deadline);
    locate_in(&nodes, process_memory(), root, anchor, deadline)
}

pub(crate) fn locate(
    bus: &SharedBus,
    root: &ShellRoot,
    landmark: Landmark,
    deadline: Deadline,
) -> Result<AtspiElement, AdapterError> {
    let located = locate_anchor(bus, root, Anchor::Landmark(landmark), deadline)?;
    let element = match landmark {
        Landmark::DateMenu => located.anchor.element,
        _ => located.container.element,
    };
    Ok(element)
}

#[cfg(test)]
#[path = "container_tests.rs"]
mod tests;
