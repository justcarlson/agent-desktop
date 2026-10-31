//! Compositor toplevels that no accessibility frame represents. They are
//! listed and focused by compositor id, with `accessible: false`.

use agent_desktop_core::{ProcessId, WindowInfo, WindowState};

use super::compositor_window_id::encode;
use super::inventory::Inventory;
use crate::system::hyprland::clients::desktop_state::DesktopState;
use crate::system::hyprland::clients::hypr_client::HyprClient;

/// The windows of every toplevel whose client `represented` rejects, in
/// client order, named by ids issued for `compositor`. A window is focused
/// only when it is the active window and its workspace is in view, as the
/// focus proof requires. A toplevel whose owner has
/// no readable instance token is left out, because it could not be
/// verified later.
pub(crate) fn compositor_windows(
    inventory: Inventory<'_>,
    desktop: &DesktopState,
    compositor: (u32, u64),
    represented: impl Fn(&HyprClient) -> bool,
    instance_of: impl Fn(u32) -> Option<String>,
) -> Vec<WindowInfo> {
    inventory
        .clients
        .iter()
        .filter(|client| !represented(client))
        .filter_map(|client| {
            let toplevel = inventory
                .toplevels
                .iter()
                .find(|toplevel| toplevel.address == client.address)?;
            let id = encode(compositor.0, compositor.1, &toplevel.identifier)?;
            let instance = instance_of(client.pid)?;
            Some(WindowInfo {
                id,
                title: client.title.clone(),
                app: client.class.clone(),
                pid: ProcessId::new(client.pid),
                process_instance: Some(instance),
                bounds: Some(client.geometry),
                state: WindowState {
                    is_focused: desktop.active == Some(client.address)
                        && desktop.in_view(&client.workspace),
                    accessible: false,
                    minimized: None,
                    visible: Some(client.shown),
                },
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "compositor_windows_tests.rs"]
mod tests;
