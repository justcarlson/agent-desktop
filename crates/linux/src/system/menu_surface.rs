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

#[expect(
    dead_code,
    reason = "read by the menu surface listing, not by the wait"
)]
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

#[expect(
    dead_code,
    reason = "the menu surface listing of the same detector calls it"
)]
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

/// The menu surface kinds this session can snapshot.
pub(crate) fn supported() -> Vec<SnapshotSurface> {
    Vec::new()
}

/// The popups open in a window and the top-level popups of its application.
pub(crate) fn open_popups(
    _bus: &SharedBus,
    _owner: &WindowInfo,
    _deadline: Deadline,
) -> Result<Vec<MenuPopup>, AdapterError> {
    Ok(Vec::new())
}

/// One `menu` entry for each open popup.
pub(crate) fn entries(_popups: &[MenuPopup], _owner: &WindowInfo) -> Vec<SurfaceInfo> {
    Vec::new()
}

#[cfg(test)]
#[path = "menu_surface_tests.rs"]
mod tests;
