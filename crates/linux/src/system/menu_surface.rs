use std::time::{Duration, Instant};

use agent_desktop_core::{
    AdapterError, Deadline, ErrorCode, SnapshotSurface, SurfaceInfo, WindowInfo,
};

pub(crate) use super::menu_popup::MenuFamily;
use super::menu_popup::{self, Node};
use super::menu_reader::BusReader;
use super::{SharedBus, window_id};
use crate::tree::AtspiElement;
use crate::tree::menu_context::MenuContext;

const SETTLE: Duration = Duration::from_millis(500);
const POLL_GAP: Duration = Duration::from_millis(40);
const MAX_POLLS: usize = 13;

pub(crate) struct MenuPopup {
    pub(crate) root: AtspiElement,
    pub(crate) family: MenuFamily,
    pub(crate) items: usize,
    pub(crate) context: MenuContext,
}

/// What a scan for open popups reads: the walk of one window, the top-level
/// children of its application root, and how to wait.
pub(super) trait PopupReader {
    fn window(&mut self) -> Result<Option<Node>, AdapterError>;
    fn top_levels(&mut self) -> Result<Vec<Node>, AdapterError>;
    fn pause(&mut self, gap: Duration);
    fn incomplete(&self) -> bool;
}

/// The menu surface kinds this session can snapshot.
pub(crate) fn supported() -> Vec<SnapshotSurface> {
    vec![SnapshotSurface::Menu]
}

fn popup_of(found: crate::system::menu_found::Found<'_>) -> MenuPopup {
    MenuPopup {
        root: found.root.payload.node.element.clone(),
        family: found.family,
        items: found.items,
        context: MenuContext::from_walk(found.root),
    }
}

fn scan(reader: &mut impl PopupReader) -> Result<Vec<MenuPopup>, AdapterError> {
    let mut popups = Vec::new();
    if let Some(frame) = reader.window()?
        && menu_popup::top_level(&frame).is_none()
    {
        popups.extend(menu_popup::in_window(&frame).into_iter().map(popup_of));
    }
    let windows = reader.top_levels()?;
    popups.extend(
        windows
            .iter()
            .filter_map(menu_popup::top_level)
            .map(popup_of),
    );
    Ok(popups)
}

/// Scans once, and again while nothing is open and `settle` allows, until
/// the bound ends. A popup of GTK 4 shows about 270 ms after the action that
/// opened it, so a scan right after the action sees nothing. The wait is
/// bounded by the poll count, by the settle time and by the deadline, never a
/// fixed sleep, and it ends at the first scan that finds a popup.
pub(super) fn find_popups(
    reader: &mut impl PopupReader,
    settle: bool,
    deadline: Deadline,
) -> Result<Vec<MenuPopup>, AdapterError> {
    let started = Instant::now();
    let mut popups = scan(reader)?;
    for _ in 0..MAX_POLLS {
        let waits = settle
            && popups.is_empty()
            && started.elapsed() + POLL_GAP < SETTLE
            && deadline.remaining() > POLL_GAP;
        if !waits {
            break;
        }
        reader.pause(POLL_GAP);
        popups = scan(reader)?;
    }
    Ok(popups)
}

fn inventory(
    reader: &mut impl PopupReader,
    settle: bool,
    deadline: Deadline,
) -> Result<Vec<MenuPopup>, AdapterError> {
    let popups = find_popups(reader, settle, deadline)?;
    if reader.incomplete() {
        return Err(AdapterError::timeout(
            "The menu inventory could not be read in full",
        ));
    }
    Ok(popups)
}

pub(super) fn vanished(error: &AdapterError) -> bool {
    matches!(
        error.code,
        ErrorCode::ElementNotFound | ErrorCode::AppNotFound | ErrorCode::WindowNotFound
    )
}

/// The popups open in the window at `path` on `bus_name` and the top-level
/// popups of its application. A window that vanished while it was read has
/// no popups.
pub(crate) fn popups_at(
    bus: &SharedBus,
    window: &AtspiElement,
    settle: bool,
    deadline: Deadline,
) -> Result<Vec<MenuPopup>, AdapterError> {
    let mut reader = BusReader::new(bus, window, deadline);
    match inventory(&mut reader, settle, deadline) {
        Err(error) if vanished(&error) => Ok(Vec::new()),
        other => other,
    }
}

pub(crate) fn popups_of(
    bus: &SharedBus,
    owner: &WindowInfo,
    settle: bool,
    deadline: Deadline,
) -> Result<Vec<MenuPopup>, AdapterError> {
    let (bus_name, path) = window_id::decode(&owner.id)?;
    let window = AtspiElement::new(bus_name, path, owner.pid.get());
    popups_at(bus, &window, settle, deadline)
}

/// One `menu` entry for the outermost open popup.
pub(crate) fn entry(popup: &MenuPopup, owner: &WindowInfo) -> SurfaceInfo {
    SurfaceInfo {
        id: owner.id.clone(),
        kind: "menu".to_owned(),
        title: None,
        item_count: Some(popup.items),
        unclassified: Vec::new(),
    }
}

/// The `menu` entries of one process's windows. A popup that is a top-level
/// window of the application belongs to no window in particular, and the
/// window that held the focus loses it to the popup, so it is listed once, on
/// the first window that sees it, and the windows after that skip it.
pub(crate) fn surfaces_of(
    bus: &SharedBus,
    owners: &[&WindowInfo],
    deadline: Deadline,
) -> Result<Vec<SurfaceInfo>, AdapterError> {
    entries_across(owners, |owner| {
        let (bus_name, path) = window_id::decode(&owner.id)?;
        let window = AtspiElement::new(bus_name, path, owner.pid.get());
        inventory(
            &mut BusReader::new(bus, &window, deadline),
            super::action_clock::settles(owner.state.is_focused),
            deadline,
        )
    })
}

fn entries_across(
    owners: &[&WindowInfo],
    mut read: impl FnMut(&WindowInfo) -> Result<Vec<MenuPopup>, AdapterError>,
) -> Result<Vec<SurfaceInfo>, AdapterError> {
    let mut listed = Vec::new();
    let mut claimed = false;
    for owner in owners {
        if let Some(popup) = read(owner)?
            .into_iter()
            .find(|popup| !(claimed && popup.family.is_top_level()))
        {
            claimed |= popup.family.is_top_level();
            listed.push(entry(&popup, owner));
        }
    }
    Ok(listed)
}

#[cfg(test)]
#[path = "menu_surface_tests.rs"]
mod tests;

pub(crate) fn roots_below(
    bus: &SharedBus,
    opener: &AtspiElement,
    deadline: Deadline,
) -> Option<Vec<AtspiElement>> {
    use crate::tree::observe_menu::{rooted, scan_walk_pruned};
    let tree = scan_walk_pruned(bus, &rooted(opener), 500, deadline).ok()?;
    roots_in_tree(&tree)
}

fn roots_in_tree(tree: &Node) -> Option<Vec<AtspiElement>> {
    use crate::tree::observe_menu::is_cut_short_beyond_pruning;
    if is_cut_short_beyond_pruning(tree) || !popup_read_complete(tree) {
        return None;
    }
    Some(
        menu_popup::in_window(tree)
            .into_iter()
            .map(|popup| popup.root.payload.node.element.address())
            .collect(),
    )
}

fn popup_read_complete(node: &Node) -> bool {
    node.payload.node.traits.role.present().is_some()
        && node.payload.node.traits.states.present().is_some()
        && !node.payload.node.children.is_failed()
        && node.children.iter().all(popup_read_complete)
}
