use agent_desktop_core::{Rect, WindowInfo, WindowState};

use super::listed_window::ListedWindow;
use super::registry_app::RegistryApp;
use super::toolkit::reports_global_position;
use super::window_filter::{frame_state, is_bubble, is_menu_popup_frame, is_window_shape};
use super::window_id;
use super::x11::FrameFacts;
use crate::tree::RawNode;
use crate::tree::role_numbers::DIALOG;

fn listed(
    index: usize,
    app: &RegistryApp,
    node: &RawNode,
    siblings: &[RawNode],
) -> Option<ListedWindow> {
    if !is_window_shape(
        &node.traits.role,
        &node.traits.interfaces,
        &node.traits.states,
    ) {
        return None;
    }
    let title = node.properties.name.present().cloned().unwrap_or_default();
    let class = node
        .attributes
        .present()
        .and_then(|attributes| attributes.get("class"))
        .map(String::as_str);
    if is_bubble(class, &title) || is_menu_popup_frame(class, &title) {
        return None;
    }
    let id = window_id::encode(node.element.bus_name(), node.element.path())?;
    let state = frame_state(&node.traits.states);
    let global_position =
        reports_global_position(app.toolkit.name.as_deref(), app.toolkit.version.as_deref());
    Some(ListedWindow {
        app: index,
        info: WindowInfo {
            id,
            title: title.clone(),
            app: app.name.clone(),
            pid: app.root.pid().into(),
            process_instance: Some(app.facts.instance.clone()),
            bounds: global_position
                .then(|| node.extents.present().copied().map(Rect::from))
                .flatten(),
            state: WindowState {
                is_focused: false,
                accessible: true,
                minimized: state.minimized,
                visible: state.visible,
            },
        },
        active: state.active == Some(true),
        modal: state.modal == Some(true),
        dialog: node.traits.role.present() == Some(&DIALOG),
        frame: FrameFacts {
            size_identity: super::frame_identity::permits_size_match(
                node,
                siblings,
                &app.toolkit,
                node.extents.present().copied(),
            ),
            pid: app.root.pid(),
            title,
            extents: node.extents.present().copied(),
            global_position,
        },
        sheet: crate::tree::surfaces::is_sheet(node),
        unshown: state.visible == Some(false) && state.minimized == Some(false),
    })
}

/// The windows among the frames read for each selected application. `apps`
/// pairs a position in the caller's list with the application, in the order
/// `frames` was read.
pub(crate) fn collect(
    apps: &[(usize, &RegistryApp)],
    frames: &[Vec<RawNode>],
) -> Vec<ListedWindow> {
    apps.iter()
        .zip(frames)
        .flat_map(|((index, app), nodes)| {
            nodes
                .iter()
                .filter_map(|node| listed(*index, app, node, nodes))
        })
        .collect()
}

#[cfg(test)]
#[path = "window_listing_tests.rs"]
mod tests;
