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
        Anchor::Landmark(Landmark::MessageList | Landmark::TrayPanel | Landmark::DockPanel) => 2,
        Anchor::Landmark(Landmark::SystemMenu) => 6,
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

fn locate_in<N: Nodes>(
    nodes: &N,
    root: &ShellRoot,
    anchor: Anchor,
    deadline: Deadline,
) -> Result<Located, AdapterError> {
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
    Ok(Located {
        anchor: found,
        container,
    })
}

pub(crate) fn locate_anchor(
    bus: &SharedBus,
    root: &ShellRoot,
    anchor: Anchor,
    deadline: Deadline,
) -> Result<Located, AdapterError> {
    let connection = bus.get(deadline)?.inner().clone();
    let nodes = LiveNodes::new(connection, deadline);
    locate_in(&nodes, root, anchor, deadline)
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
