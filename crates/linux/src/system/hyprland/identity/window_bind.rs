//! Binds a listed window to its one compositor toplevel: by its compositor
//! id for a compositor-only window, by its frame's evidence otherwise.

use agent_desktop_core::{AdapterError, WindowInfo};

use super::bound_toplevel::BoundToplevel;
use super::compositor_window_id::{decode, is_compositor_id};
use super::identity_bind::bind_compositor;
use super::identity_refusal::IdentityRefusal;
use super::inventory::Inventory;
use super::window_evidence::WindowEvidence;
use crate::system::window::compositor_identity::{bound_toplevel, resolution_error};
use crate::system::window::window_id;

/// The one toplevel `win` is, read from `inventory` of the compositor
/// identified by `compositor` (its pid and start time). `live_instance` is
/// the instance token of the window's process as it runs now.
pub(crate) fn bind_window(
    win: &WindowInfo,
    inventory: Inventory<'_>,
    live_instance: Option<&str>,
    compositor: (u32, u64),
) -> Result<BoundToplevel, AdapterError> {
    let evidence = WindowEvidence {
        pid: win.pid.get(),
        instance: win.process_instance.clone(),
        title: win.title.clone(),
        size: win.bounds.map(|bounds| (bounds.width, bounds.height)),
    };
    let address = if is_compositor_id(&win.id) {
        let id = decode(&win.id).ok_or_else(|| window_id::invalid(&win.id))?;
        bind_compositor(&id, &evidence, compositor, live_instance, inventory)
            .map_err(resolution_error)?
    } else {
        bound_toplevel(&evidence, live_instance, inventory)?
    };
    bound_at(address, inventory).map_err(resolution_error)
}

fn bound_at(address: u64, inventory: Inventory<'_>) -> Result<BoundToplevel, IdentityRefusal> {
    let client = inventory
        .clients
        .iter()
        .find(|client| client.address == address)
        .ok_or(IdentityRefusal::ToplevelGone)?;
    let toplevel = inventory
        .toplevels
        .iter()
        .find(|toplevel| toplevel.address == address)
        .ok_or(IdentityRefusal::ToplevelGone)?;
    Ok(BoundToplevel {
        client: client.clone(),
        toplevel: toplevel.clone(),
    })
}

#[cfg(test)]
#[path = "window_bind_tests.rs"]
mod tests;
