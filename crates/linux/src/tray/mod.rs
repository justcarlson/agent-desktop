#![expect(
    dead_code,
    reason = "the shell observer that calls read and the view is not on this branch"
)]

use agent_desktop_core::{
    ActionRequest, ActionResult, AdapterError, Deadline, ElementState, HitTestResult,
    InteractionLease, LiveElement, NativeHandle, Point, Rect, RefEntry,
};

mod activate;
mod call;
mod capability;
mod execute;
mod identity;
mod introspect;
mod item_props;
mod live;
mod menu_event;
mod menu_layout;
mod menu_nodes;
mod menu_resolve;
mod node_evidence;
mod panel_correlate;
mod registration;
mod resolve;
pub(crate) mod view;
mod watcher;
mod world;

#[cfg(test)]
mod fit_tests;
#[cfg(test)]
mod live_rig;
#[cfg(test)]
mod live_scripts;
#[cfg(test)]
mod live_tests;

pub(crate) fn read(deadline: Deadline) -> Result<view::TrayView, AdapterError> {
    view::read(deadline)
}

/// A tray item or one of its menu entries, named by D-Bus identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrayTarget {
    pub(crate) owner: String,
    pub(crate) object_path: String,
    pub(crate) menu_path: Option<String>,
    pub(crate) entry_id: Option<i32>,
    pub(crate) revision: Option<u32>,
    pub(crate) label_path: Vec<String>,
}

pub(crate) fn target_of(handle: &NativeHandle) -> Option<&TrayTarget> {
    handle.downcast_ref::<TrayTarget>()
}

/// Whether a stored ref names a tray item or one of its menu entries.
pub(crate) fn owns(entry: &RefEntry) -> bool {
    entry
        .identity
        .native_id
        .as_ref()
        .is_some_and(|id| identity::is_tray_id(&id.value))
}

pub(crate) fn resolve(entry: &RefEntry, deadline: Deadline) -> Result<NativeHandle, AdapterError> {
    resolve::resolve(entry, deadline)
}

pub(crate) fn execute(
    target: &TrayTarget,
    request: ActionRequest,
    lease: &InteractionLease,
) -> Result<ActionResult, AdapterError> {
    execute::execute(target, request, lease)
}

pub(crate) fn live_actions(
    target: &TrayTarget,
    deadline: Deadline,
) -> Result<Option<Vec<String>>, AdapterError> {
    live::actions(target, deadline).map(Some)
}

pub(crate) fn live_state(
    target: &TrayTarget,
    deadline: Deadline,
) -> Result<Option<ElementState>, AdapterError> {
    live::state(target, deadline).map(Some)
}

pub(crate) fn live_element(
    target: &TrayTarget,
    deadline: Deadline,
) -> Result<LiveElement, AdapterError> {
    live::element(target, deadline)
}

pub(crate) fn bounds(
    target: &TrayTarget,
    deadline: Deadline,
) -> Result<Option<Rect>, AdapterError> {
    live::bounds(target, deadline)
}

pub(crate) fn hit_test(
    target: &TrayTarget,
    point: Point,
    deadline: Deadline,
) -> Result<HitTestResult, AdapterError> {
    let rect = live::bounds(target, deadline)?;
    let displays = crate::system::shell::hit::display_extents(deadline);
    Ok(reach(rect, &point, displays.as_deref()))
}

/// A tray item is reached when the point lies inside its panel node's extents
/// on a display; an item with no panel node and a menu entry cannot be told.
fn reach(
    rect: Option<Rect>,
    point: &Point,
    displays: Option<&[crate::tree::Extents]>,
) -> HitTestResult {
    let extents = rect.map(|rect| crate::tree::Extents {
        x: rect.x as i32,
        y: rect.y as i32,
        width: rect.width as i32,
        height: rect.height as i32,
    });
    crate::system::shell::hit::decide(extents, point, displays)
}

#[cfg(test)]
mod reach_tests {
    use super::*;
    use crate::tree::Extents;

    const DISPLAY: Extents = Extents {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };

    fn item() -> Option<Rect> {
        Some(Rect {
            x: 1800.0,
            y: 0.0,
            width: 30.0,
            height: 30.0,
        })
    }

    #[test]
    fn a_point_inside_the_item_reaches_it() {
        let point = Point { x: 1810.0, y: 10.0 };
        assert_eq!(
            reach(item(), &point, Some(&[DISPLAY])),
            HitTestResult::ReachesTarget
        );
    }

    #[test]
    fn a_point_outside_the_item_is_unknown() {
        let point = Point { x: 100.0, y: 500.0 };
        assert_eq!(
            reach(item(), &point, Some(&[DISPLAY])),
            HitTestResult::Unknown
        );
    }

    #[test]
    fn an_item_with_no_extents_is_unknown() {
        let point = Point { x: 1810.0, y: 10.0 };
        assert_eq!(
            reach(None, &point, Some(&[DISPLAY])),
            HitTestResult::Unknown
        );
    }
}
