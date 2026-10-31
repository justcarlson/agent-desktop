//! Binds a window to exactly one compositor toplevel by evidence.

use super::identity_refusal::IdentityRefusal;
use super::inventory::Inventory;
use super::window_evidence::WindowEvidence;
use crate::system::hyprland::clients::hypr_client::HyprClient;

const SIZE_TOLERANCE: f64 = 1.0;

/// The Hyprland address of the one toplevel of `frame`'s process whose title is exactly the frame's
/// title. Twins are told apart by size only when exactly one size matches;
/// a sole title match binds whatever its size, because tiling resizes a
/// client without resizing its accessibility extents. A twin with no
/// toplevel mapping still counts, so a missing mapping never hands the frame
/// to its sibling. Never chooses by enumeration order.
pub(crate) fn bind_frame(
    frame: &WindowEvidence,
    live_instance: Option<&str>,
    inventory: Inventory<'_>,
) -> Result<u64, IdentityRefusal> {
    check_process(frame, live_instance)?;
    let matches: Vec<&HyprClient> = inventory
        .clients
        .iter()
        .filter(|client| client.pid == frame.pid && client.title == frame.title)
        .collect();
    let chosen = match matches.as_slice() {
        [] => return Err(IdentityRefusal::NoCandidate),
        [only] => *only,
        _ => pick_by_size(&matches, frame.size)?,
    };
    inventory
        .toplevels
        .iter()
        .any(|toplevel| toplevel.address == chosen.address)
        .then_some(chosen.address)
        .ok_or(IdentityRefusal::NoCandidate)
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

fn pick_by_size<'a>(
    matches: &[&'a HyprClient],
    size: Option<(f64, f64)>,
) -> Result<&'a HyprClient, IdentityRefusal> {
    let sized: Vec<&HyprClient> = match size {
        Some(size) => matches
            .iter()
            .copied()
            .filter(|client| same_size(client, size))
            .collect(),
        None => Vec::new(),
    };
    if let [only] = sized.as_slice() {
        return Ok(only);
    }
    let mut named: Vec<(u64, String)> = matches
        .iter()
        .map(|client| (client.address, client.workspace.name.clone()))
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
