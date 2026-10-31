//! Compositor toplevels that no accessibility frame represents. They are
//! listed and focused by compositor id, with `accessible: false`.

use agent_desktop_core::{ProcessId, WindowInfo, WindowState};

use super::compositor_window_id::encode;
use super::inventory::Inventory;

/// The windows of every toplevel that `represented` rejects for its owner's
/// pid and title, in client order, named by ids issued for `compositor`. A toplevel whose owner has
/// no readable instance token is left out, because it could not be
/// verified later.
pub(crate) fn compositor_windows(
    inventory: Inventory<'_>,
    active: Option<u64>,
    compositor: (u32, u64),
    represented: impl Fn(u32, &str) -> bool,
    instance_of: impl Fn(u32) -> Option<String>,
) -> Vec<WindowInfo> {
    inventory
        .clients
        .iter()
        .filter(|client| !represented(client.pid, &client.title))
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
                    is_focused: active == Some(client.address),
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
