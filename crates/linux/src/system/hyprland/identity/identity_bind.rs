//! Binds a window to exactly one compositor toplevel by evidence.

use super::bound_toplevel::BoundToplevel;
use super::compositor_window_id::CompositorWindowId;
use super::identity_refusal::IdentityRefusal;
use super::inventory::Inventory;
use super::window_evidence::WindowEvidence;
use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::wayland::toplevel::toplevel_view::ToplevelView;

const SIZE_TOLERANCE: f64 = 1.0;

/// The one toplevel of `frame`'s process whose title is exactly the frame's
/// title. Twins are told apart by size only when exactly one size matches;
/// a sole title match binds whatever its size, because tiling resizes a
/// client without resizing its accessibility extents. Never chooses by
/// enumeration order.
pub(crate) fn bind_frame(
    frame: &WindowEvidence,
    live_instance: Option<&str>,
    inventory: Inventory<'_>,
) -> Result<BoundToplevel, IdentityRefusal> {
    check_process(frame, live_instance)?;
    let candidates: Vec<BoundToplevel> = inventory
        .clients
        .iter()
        .filter(|client| client.pid == frame.pid && client.title == frame.title)
        .filter_map(|client| with_toplevel(client, inventory.toplevels))
        .collect();
    match candidates.len() {
        0 => Err(IdentityRefusal::NoCandidate),
        1 => Ok(candidates
            .into_iter()
            .next()
            .ok_or(IdentityRefusal::NoCandidate)?),
        _ => pick_by_size(candidates, frame.size),
    }
}

/// The toplevel `id` names, after checking that the compositor that issued
/// the id still runs and that `owner`'s process still owns the toplevel.
pub(crate) fn bind_compositor(
    id: &CompositorWindowId,
    owner: &WindowEvidence,
    compositor: (u32, u64),
    live_instance: Option<&str>,
    inventory: Inventory<'_>,
) -> Result<BoundToplevel, IdentityRefusal> {
    if !id.issued_by(compositor.0, compositor.1) {
        return Err(IdentityRefusal::OtherCompositor);
    }
    check_process(owner, live_instance)?;
    let toplevel = inventory
        .toplevels
        .iter()
        .find(|toplevel| toplevel.identifier == id.identifier)
        .ok_or(IdentityRefusal::ToplevelGone)?;
    let client = inventory
        .clients
        .iter()
        .find(|client| client.address == toplevel.address)
        .ok_or(IdentityRefusal::ToplevelGone)?;
    if client.pid != owner.pid {
        return Err(IdentityRefusal::ProcessChanged);
    }
    Ok(BoundToplevel {
        client: client.clone(),
        toplevel: toplevel.clone(),
    })
}

fn check_process(
    evidence: &WindowEvidence,
    live_instance: Option<&str>,
) -> Result<(), IdentityRefusal> {
    match (evidence.instance.as_deref(), live_instance) {
        (Some(recorded), Some(live)) if recorded == live => Ok(()),
        _ => Err(IdentityRefusal::ProcessChanged),
    }
}

fn with_toplevel(client: &HyprClient, toplevels: &[ToplevelView]) -> Option<BoundToplevel> {
    toplevels
        .iter()
        .find(|toplevel| toplevel.address == client.address)
        .map(|toplevel| BoundToplevel {
            client: client.clone(),
            toplevel: toplevel.clone(),
        })
}

fn pick_by_size(
    candidates: Vec<BoundToplevel>,
    size: Option<(f64, f64)>,
) -> Result<BoundToplevel, IdentityRefusal> {
    let mut sized: Vec<&BoundToplevel> = match size {
        Some(size) => candidates
            .iter()
            .filter(|candidate| same_size(&candidate.client, size))
            .collect(),
        None => Vec::new(),
    };
    if sized.len() == 1 {
        if let Some(only) = sized.pop() {
            return Ok(only.clone());
        }
    }
    let mut named: Vec<(u64, String)> = candidates
        .iter()
        .map(|candidate| {
            (
                candidate.client.address,
                candidate.client.workspace.name.clone(),
            )
        })
        .collect();
    named.sort();
    Err(IdentityRefusal::Ambiguous(named))
}

fn same_size(client: &HyprClient, size: (f64, f64)) -> bool {
    (client.geometry.width - size.0).abs() <= SIZE_TOLERANCE
        && (client.geometry.height - size.1).abs() <= SIZE_TOLERANCE
}

#[cfg(test)]
#[path = "identity_bind_tests.rs"]
mod tests;
