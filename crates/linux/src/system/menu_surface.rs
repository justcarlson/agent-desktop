use std::time::{Duration, Instant};

use agent_desktop_core::{
    AdapterError, Deadline, ErrorCode, SnapshotSurface, SurfaceInfo, WindowInfo,
};

pub(crate) use super::menu_popup::MenuFamily;
use super::menu_popup::{self, Node};
use super::menu_reader::BusReader;
use super::{SharedBus, window_id};
use crate::tree::AtspiElement;

const SETTLE: Duration = Duration::from_millis(500);
const POLL_GAP: Duration = Duration::from_millis(40);
const MAX_POLLS: usize = 13;

pub(crate) struct MenuPopup {
    pub(crate) root: AtspiElement,
    pub(crate) family: MenuFamily,
    pub(crate) items: usize,
}

/// What a scan for open popups reads: the walk of one window, the top-level
/// children of its application root, and how to wait.
pub(super) trait PopupReader {
    fn window(&mut self) -> Result<Option<Node>, AdapterError>;
    fn top_levels(&mut self) -> Result<Vec<Node>, AdapterError>;
    fn pause(&mut self, gap: Duration);
}

/// The menu surface kinds this session can snapshot.
pub(crate) fn supported() -> Vec<SnapshotSurface> {
    vec![SnapshotSurface::Menu]
}

fn popup_of(found: menu_popup::Found<'_>) -> MenuPopup {
    MenuPopup {
        root: found.root.payload.node.element.clone(),
        family: found.family,
        items: found.items,
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
    match find_popups(&mut reader, settle, deadline) {
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

/// The popups open in a window and the top-level popups of its application,
/// outermost first. Only a focused window waits for a popup to appear: an
/// action runs in the window that has focus.
pub(crate) fn open_popups(
    bus: &SharedBus,
    owner: &WindowInfo,
    deadline: Deadline,
) -> Result<Vec<MenuPopup>, AdapterError> {
    popups_of(bus, owner, owner.state.is_focused, deadline)
}

/// One `menu` entry for the outermost open popup.
pub(crate) fn entries(popups: &[MenuPopup], owner: &WindowInfo) -> Vec<SurfaceInfo> {
    popups
        .first()
        .map(|popup| SurfaceInfo {
            id: owner.id.clone(),
            kind: "menu".to_owned(),
            title: None,
            item_count: Some(popup.items),
            unclassified: Vec::new(),
        })
        .into_iter()
        .collect()
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
    entries_across(owners, |owner| open_popups(bus, owner, deadline))
}

fn entries_across(
    owners: &[&WindowInfo],
    mut read: impl FnMut(&WindowInfo) -> Result<Vec<MenuPopup>, AdapterError>,
) -> Result<Vec<SurfaceInfo>, AdapterError> {
    let mut listed = Vec::new();
    let mut claimed = false;
    for owner in owners {
        let popups: Vec<MenuPopup> = read(owner)?
            .into_iter()
            .filter(|popup| !(claimed && popup.family.is_top_level()))
            .collect();
        claimed |= popups
            .first()
            .is_some_and(|popup| popup.family.is_top_level());
        listed.extend(entries(&popups, owner));
    }
    Ok(listed)
}

#[cfg(test)]
#[path = "menu_surface_tests.rs"]
mod tests;
