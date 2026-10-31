//! Binds a listed window to its one compositor toplevel: by its compositor
//! id for a compositor-only window, by its frame's evidence otherwise.

use agent_desktop_core::{AdapterError, WindowInfo};

use super::bound_toplevel::BoundToplevel;
use super::compositor_window_id::{decode, is_compositor_id};
use super::identity_bind::{bind_compositor, bind_frame};
use super::identity_error::identity_error;
use super::identity_refusal::IdentityRefusal;
use super::inventory::Inventory;
use super::window_evidence::WindowEvidence;

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
    let bound = if is_compositor_id(&win.id) {
        match decode(&win.id) {
            Some(id) => bind_compositor(&id, &evidence, compositor, live_instance, inventory),
            None => Err(IdentityRefusal::ToplevelGone),
        }
    } else {
        bind_frame(&evidence, live_instance, inventory)
    };
    bound.map_err(identity_error)
}
