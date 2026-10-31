//! Binds a window for capture and rechecks the binding once its frame is
//! taken. Clients and toplevels are joined only by address, and Hyprland
//! can hand a closed window's address to a new one, so the clients are read
//! on both sides of the toplevel read and again after the frame.

use std::cell::RefCell;

use agent_desktop_core::{AdapterError, WindowInfo};

use super::toplevel_reads::ToplevelReads;
use crate::system::hyprland::identity::bound_toplevel::BoundToplevel;
use crate::system::hyprland::identity::inventory::Inventory;
use crate::system::hyprland::identity::inventory_read::stable_inventory;
use crate::system::hyprland::identity::window_bind::bind_window;

/// The one toplevel `win` is, from clients that name the same process and
/// class on both sides of the toplevel read.
pub(crate) fn bind_stable(
    win: &WindowInfo,
    reads: &mut impl ToplevelReads,
) -> Result<BoundToplevel, AdapterError> {
    let shared = RefCell::new(&mut *reads);
    let (clients, toplevels) = stable_inventory(
        || shared.borrow_mut().clients(),
        || shared.borrow_mut().toplevels(),
    )?;
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };
    let live = reads.live_instance(win.pid.get());
    bind_window(win, inventory, live.as_deref(), reads.compositor())
}

/// Whether `bound` still is `win` after the frame: the compositor still has
/// its toplevel at its address, that address still names the bound process
/// and class, and the process still runs as the instance `win` recorded.
pub(crate) fn still_bound_to(
    win: &WindowInfo,
    bound: &BoundToplevel,
    reads: &mut impl ToplevelReads,
) -> Result<bool, AdapterError> {
    let address = bound.client.address;
    let toplevel = reads
        .fresh_toplevels()?
        .iter()
        .any(|view| view.identifier == bound.toplevel.identifier && view.address == address);
    let client = reads.clients()?.iter().any(|client| {
        client.address == address
            && client.pid == bound.client.pid
            && client.class == bound.client.class
    });
    let live = reads.live_instance(bound.client.pid);
    let process = live.is_some() && live == win.process_instance;
    Ok(toplevel && client && process)
}

#[cfg(test)]
#[path = "capture_bind_tests.rs"]
mod tests;
