//! Live receiver evidence from the bound instance: its clients, its layer
//! surfaces and what each monitor shows, read for each check.

use agent_desktop_core::{AdapterError, Deadline, ErrorCode, Point};

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

/// Reads every client, every layer surface and what each monitor shows. A
/// read that runs out of time is [`ReceiverRefusal::EvidenceTimedOut`].
pub(crate) fn read_stack(
    binding: &HyprlandBinding,
    deadline: Deadline,
) -> Result<ReceiverStack, ReceiverRefusal> {
    let missing = |check| ReceiverRefusal::EvidenceMissing { check };
    let clients = evidence(
        bound_request(binding, CLIENTS_COMMAND, deadline),
        "client_stack",
    )?;
    let clients = parse_stack_clients(&clients).ok_or(missing("client_stack"))?;
    let layers = evidence(
        bound_request(binding, LAYERS_COMMAND, deadline),
        "layer_stack",
    )?;
    let layers = parse_layers(&layers).ok_or(missing("layer_stack"))?;
    let desktop = evidence(read_desktop(binding, deadline), "workspaces")?;
    Ok(ReceiverStack {
        clients,
        layers,
        desktop,
    })
}

/// A failed read is missing evidence, except one that ran out of time.
fn evidence<T>(read: Result<T, AdapterError>, check: &'static str) -> Result<T, ReceiverRefusal> {
    read.map_err(|error| match error.code {
        ErrorCode::Timeout => ReceiverRefusal::EvidenceTimedOut { check },
        _ => ReceiverRefusal::EvidenceMissing { check },
    })
}

#[cfg(test)]
#[path = "receiver_read_tests.rs"]
mod tests;
