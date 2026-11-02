//! Whether a ref pointer action on the bound Hyprland instance has proof of
//! which surface takes its point. The compositor's stack proves only which
//! toplevel takes a point; the client then routes the point inside its own
//! surfaces, and the stack does not show where it goes. On a native Wayland
//! client an in-window overlay or a same-client popup took ref clicks while
//! the stack and the accessibility descent both reported the target
//! (probes/linux/FINDINGS.md L47-4, L47-5). On an XWayland client the same
//! covers took 12 of 12 overlay clicks and every GTK popup click while the
//! stack proof passed (L45-2). No other receiver proof exists, so every
//! target refuses before any pointer or modifier event: a native Wayland
//! target with cause `native_wayland_client`, an XWayland target with
//! cause `receiver_unproven`.

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};
use serde_json::{Value, json};

use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::clients::client_parse::parse_address;
use crate::system::hyprland::ipc::bound_request::bound_request;
use crate::system::hyprland::receiver::receiver_refusal::ReceiverRefusal;

const XWAYLAND_EVIDENCE: &str = "client_xwayland";

/// Refuses a ref pointer action on the target window, named native Wayland
/// or XWayland by the bound instance's `j/clients`.
pub(crate) fn refuse_unproven_receiver(
    binding: &HyprlandBinding,
    address: u64,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    check_unproven_read(bound_request(binding, "j/clients", deadline), address)
}

/// The same check over the `j/clients` read itself. A failed read refuses as
/// missing evidence, or as timed out evidence when it ran out of time.
pub(crate) fn check_unproven_read(
    read: Result<Vec<u8>, AdapterError>,
    address: u64,
) -> Result<(), AdapterError> {
    let clients = read.map_err(|error| {
        let check = XWAYLAND_EVIDENCE;
        match error.code {
            ErrorCode::Timeout => ReceiverRefusal::EvidenceTimedOut { check },
            _ => ReceiverRefusal::EvidenceMissing { check },
        }
        .to_error()
    })?;
    check_unproven_receiver(&clients, address)
}

/// The same check over a `j/clients` reply. A reply that does not name the
/// target with an `xwayland` flag refuses as missing evidence.
pub(crate) fn check_unproven_receiver(clients: &[u8], address: u64) -> Result<(), AdapterError> {
    match xwayland_flag(clients, address) {
        Some(true) => Err(unproven_refusal()),
        Some(false) => Err(native_refusal()),
        None => Err(ReceiverRefusal::EvidenceMissing {
            check: XWAYLAND_EVIDENCE,
        }
        .to_error()),
    }
}

/// The refusal for a ref pointer gesture read from a `j/clients` reply and
/// the saved entry's process, with no element resolved: native Wayland when
/// every toplevel of the process is native, else `receiver_unproven`, which
/// also answers a missing reply or a process with no toplevel.
pub(crate) fn entry_receiver_refusal(clients: Option<&[u8]>, pid: u32) -> AdapterError {
    match clients.and_then(|clients| process_xwayland_flag(clients, pid)) {
        Some(false) => native_refusal(),
        _ => unproven_refusal(),
    }
}

fn process_xwayland_flag(clients: &[u8], pid: u32) -> Option<bool> {
    let clients: Vec<Value> = serde_json::from_slice(clients).ok()?;
    let mut flags = clients
        .iter()
        .filter(|client| client.get("pid").and_then(Value::as_u64) == Some(u64::from(pid)))
        .map(|client| client.get("xwayland").and_then(Value::as_bool));
    let first = flags.next()??;
    flags.all(|flag| flag == Some(first)).then_some(first)
}

fn xwayland_flag(clients: &[u8], address: u64) -> Option<bool> {
    let clients: Vec<Value> = serde_json::from_slice(clients).ok()?;
    let client = clients.iter().find(|client| {
        client
            .get("address")
            .and_then(Value::as_str)
            .and_then(parse_address)
            == Some(address)
    })?;
    client.get("xwayland")?.as_bool()
}

fn native_refusal() -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "The target is a native Wayland window, and Hyprland cannot prove which of its surfaces takes the pointer",
    )
    .with_details(json!({
        "kind": "receiver_evidence",
        "check": "hyprland_stack",
        "cause": "native_wayland_client",
        "physical_delivery_started": false,
    }))
    .with_disposition(DeliverySemantics::not_delivered())
    .with_suggestion(
        "Run the ref action without --headed to use its accessibility action, or aim with mouse-click --xy, which reports receiver_proof: false, and read the effect from the app",
    )
}

fn unproven_refusal() -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "The target is an XWayland window, and Hyprland cannot prove which of its surfaces takes the pointer; run the ref action without --headed, or aim with --xy",
    )
    .with_details(json!({
        "kind": "receiver_evidence",
        "check": "hyprland_stack",
        "cause": "receiver_unproven",
        "physical_delivery_started": false,
    }))
    .with_disposition(DeliverySemantics::not_delivered())
    .with_suggestion(
        "Run the ref action without --headed, such as click @ref, to use its accessibility action, or aim with coordinates (mouse-click --xy, mouse-wheel --x --y, drag --from-xy --to-xy), which report receiver_proof: false, and read the effect from the app",
    )
}

#[cfg(test)]
#[path = "native_receiver_tests.rs"]
mod tests;
