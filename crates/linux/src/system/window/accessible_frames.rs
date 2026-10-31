//! The accessibility frames a listing read, and which compositor toplevels
//! they already represent.

use std::collections::HashSet;

use crate::system::hyprland::identity::identity_bind::bind_frame;
use crate::system::hyprland::identity::identity_refusal::IdentityRefusal;
use crate::system::hyprland::identity::inventory::Inventory;
use crate::system::hyprland::identity::window_evidence::WindowEvidence;
use crate::system::registry::listed_window::ListedWindow;

/// The frames a listing read, the processes it read them from, and every
/// registered process. A process the listing's filter skipped is judged by
/// registration alone.
#[derive(Debug, Default)]
pub(crate) struct AccessibleFrames {
    pub(crate) frames: Vec<WindowEvidence>,
    pub(crate) read: HashSet<u32>,
    pub(crate) registered: HashSet<u32>,
}

/// What a listed window says about itself. The size is the placed bounds
/// when placement found them, else the frame's own extents: a native
/// GTK 4 window has no global bounds after placement, and without a size its
/// same-titled twins could not be told apart.
pub(crate) fn frame_evidence(window: &ListedWindow) -> WindowEvidence {
    let extents = window
        .frame
        .extents
        .map(|extents| (f64::from(extents.width), f64::from(extents.height)));
    WindowEvidence {
        pid: window.info.pid.get(),
        instance: window.info.process_instance.clone(),
        title: window.info.title.clone(),
        size: window
            .info
            .bounds
            .map(|bounds| (bounds.width, bounds.height))
            .or(extents),
    }
}

impl AccessibleFrames {
    /// The addresses of the toplevels these frames represent. Each frame
    /// hides the toplevel it binds to by process, title and size.
    /// A frame whose twins cannot be told apart hides every twin, and a
    /// frame that cannot be bound hides the toplevels of its process that
    /// carry its title, so an accessible window is not listed twice for want
    /// of evidence. `live_instance` reads each frame's process again; a frame
    /// whose recorded process has gone or changed since it was read
    /// represents nothing, so a replacement process with the same pid and
    /// title keeps its own toplevel.
    pub(crate) fn represented(
        &self,
        inventory: Inventory<'_>,
        live_instance: impl Fn(u32) -> Option<String>,
    ) -> HashSet<u64> {
        let mut addresses: HashSet<u64> = inventory
            .clients
            .iter()
            .filter(|client| {
                !self.read.contains(&client.pid) && self.registered.contains(&client.pid)
            })
            .map(|client| client.address)
            .collect();
        for frame in &self.frames {
            let live = live_instance(frame.pid);
            if frame.instance.is_some() && live != frame.instance {
                continue;
            }
            match bind_frame(frame, live.as_deref(), inventory) {
                Ok(address) => {
                    addresses.insert(address);
                }
                Err(IdentityRefusal::Ambiguous(twins)) => {
                    addresses.extend(twins.into_iter().map(|(address, _)| address));
                }
                Err(_) => addresses.extend(
                    inventory
                        .clients
                        .iter()
                        .filter(|client| client.pid == frame.pid && client.title == frame.title)
                        .map(|client| client.address),
                ),
            }
        }
        addresses
    }
}

#[cfg(test)]
#[path = "accessible_frames_tests.rs"]
mod tests;
