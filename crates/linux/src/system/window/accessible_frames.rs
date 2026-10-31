//! The accessibility frames a listing read, and which compositor toplevels
//! they already represent.

use std::collections::HashSet;

use agent_desktop_core::WindowInfo;

use crate::system::hyprland::identity::identity_bind::bind_frame;
use crate::system::hyprland::identity::identity_refusal::IdentityRefusal;
use crate::system::hyprland::identity::inventory::Inventory;
use crate::system::hyprland::identity::window_evidence::WindowEvidence;

/// The frames a listing read, the processes it read them from, and every
/// registered process. A process the listing's filter skipped is judged by
/// registration alone.
#[derive(Debug, Default)]
pub(crate) struct AccessibleFrames {
    pub(crate) frames: Vec<WindowEvidence>,
    pub(crate) read: HashSet<u32>,
    pub(crate) registered: HashSet<u32>,
}

impl AccessibleFrames {
    /// The frames of `listed`, read from the processes in `read`.
    pub(crate) fn from_listed(
        listed: &[WindowInfo],
        read: HashSet<u32>,
        registered: HashSet<u32>,
    ) -> Self {
        let frames = listed
            .iter()
            .map(|window| WindowEvidence {
                pid: window.pid.get(),
                instance: window.process_instance.clone(),
                title: window.title.clone(),
                size: window.bounds.map(|bounds| (bounds.width, bounds.height)),
            })
            .collect();
        Self {
            frames,
            read,
            registered,
        }
    }

    /// The addresses of the toplevels these frames represent. Each frame
    /// hides the toplevel it binds to by the evidence `focus-window` uses.
    /// A frame whose twins cannot be told apart hides every twin, and a
    /// frame that cannot be bound hides the toplevels of its process that
    /// carry its title, so an accessible window is not listed twice for want
    /// of evidence.
    pub(crate) fn represented(&self, inventory: Inventory<'_>) -> HashSet<u64> {
        let mut addresses: HashSet<u64> = inventory
            .clients
            .iter()
            .filter(|client| {
                !self.read.contains(&client.pid) && self.registered.contains(&client.pid)
            })
            .map(|client| client.address)
            .collect();
        for frame in &self.frames {
            match bind_frame(frame, frame.instance.as_deref(), inventory) {
                Ok(bound) => {
                    addresses.insert(bound.client.address);
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
