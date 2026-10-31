//! On a bound Hyprland, an accessibility window resolves only when it is
//! exactly one compositor toplevel.

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};
use serde_json::{Value, json};

use super::compositor_listing::live_instance;
use crate::system::hyprland::bind::routed_binding::routed_binding;
use crate::system::hyprland::identity::identity_bind::bind_frame;
use crate::system::hyprland::identity::identity_refusal::IdentityRefusal;
use crate::system::hyprland::identity::inventory::Inventory;
use crate::system::hyprland::identity::inventory_read::read_inventory;
use crate::system::hyprland::identity::window_evidence::WindowEvidence;

/// Requires `evidence` to bind to exactly one toplevel of the bound Hyprland
/// instance. Any other route has no compositor identity to check, and a
/// refused session binding is that binding's own error.
pub(crate) fn require_toplevel(
    evidence: &WindowEvidence,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    let Some(binding) = routed_binding()? else {
        return Ok(());
    };
    let (clients, toplevels) = read_inventory(&binding, deadline)?;
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };
    bound_toplevel(evidence, live_instance(evidence.pid).as_deref(), inventory).map(|_| ())
}

/// The address of the one toplevel `evidence` binds to. A process that
/// changed and a window with no toplevel are `WINDOW_NOT_FOUND`, and
/// several candidates are `AMBIGUOUS_TARGET` naming each one. Nothing was
/// sent, so every refusal is safe to retry after a fresh `list-windows`.
pub(crate) fn bound_toplevel(
    evidence: &WindowEvidence,
    live_instance: Option<&str>,
    inventory: Inventory<'_>,
) -> Result<u64, AdapterError> {
    bind_frame(evidence, live_instance, inventory).map_err(resolution_error)
}

fn resolution_error(refusal: IdentityRefusal) -> AdapterError {
    let error = match refusal {
        IdentityRefusal::ProcessChanged => {
            not_found("The window's process is no longer the one that showed it")
        }
        IdentityRefusal::NoCandidate => {
            not_found("The compositor shows no toplevel for this window")
        }
        IdentityRefusal::Ambiguous(candidates) => ambiguous(&candidates),
    };
    error.with_disposition(DeliverySemantics::not_delivered())
}

fn not_found(message: &str) -> AdapterError {
    AdapterError::new(ErrorCode::WindowNotFound, message)
        .with_suggestion("Run 'list-windows' to see the windows that exist now")
}

fn ambiguous(candidates: &[(u64, String)]) -> AdapterError {
    let named: Vec<Value> = candidates
        .iter()
        .map(|(address, workspace)| {
            json!({"address": format!("{address:#x}"), "workspace": workspace})
        })
        .collect();
    AdapterError::new(
        ErrorCode::AmbiguousTarget,
        "Several compositor toplevels match this window",
    )
    .with_suggestion("Give the windows distinct titles or sizes, then run 'list-windows' and retry")
    .with_details(json!({ "candidates": named }))
}

#[cfg(test)]
#[path = "compositor_identity_tests.rs"]
mod tests;
