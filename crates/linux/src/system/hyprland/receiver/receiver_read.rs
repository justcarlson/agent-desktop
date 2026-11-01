//! Live receiver evidence from the bound instance: its clients, its layer
//! surfaces and what each monitor shows, read for each check.

use agent_desktop_core::{AdapterError, Deadline, Point};

use super::check_receiver::check_receiver;
use super::deny_all_overlays::DenyAllOverlays;
use super::receiver_refusal::ReceiverRefusal;
use super::stack::ReceiverStack;
use super::stack_parse::{parse_layers, parse_stack_clients};
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::clients::client_read::read_desktop;
use crate::system::hyprland::ipc::bound_request::bound_request;

const CLIENTS_COMMAND: &str = "j/clients";
const LAYERS_COMMAND: &str = "j/layers";

/// Proves that `point` reaches the toplevel at `target`, with every layer
/// surface counted as a cover. Any read that fails refuses as missing
/// evidence, before any event.
pub(crate) fn prove_receiver(
    binding: &HyprlandBinding,
    target: u64,
    point: &Point,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    let stack = read_stack(binding, deadline).map_err(|refusal| refusal.to_error())?;
    check_receiver(&stack, target, point, &DenyAllOverlays).map_err(|refusal| refusal.to_error())
}

/// Reads every client, every layer surface and what each monitor shows.
pub(crate) fn read_stack(
    binding: &HyprlandBinding,
    deadline: Deadline,
) -> Result<ReceiverStack, ReceiverRefusal> {
    let missing = |check| ReceiverRefusal::EvidenceMissing { check };
    let clients = bound_request(binding, CLIENTS_COMMAND, deadline)
        .ok()
        .and_then(|body| parse_stack_clients(&body))
        .ok_or(missing("client_stack"))?;
    let layers = bound_request(binding, LAYERS_COMMAND, deadline)
        .ok()
        .and_then(|body| parse_layers(&body))
        .ok_or(missing("layer_stack"))?;
    let desktop = read_desktop(binding, deadline).map_err(|_| missing("workspaces"))?;
    Ok(ReceiverStack {
        clients,
        layers,
        desktop,
    })
}

#[cfg(test)]
#[path = "receiver_read_tests.rs"]
mod tests;
