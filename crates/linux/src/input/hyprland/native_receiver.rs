//! Whether a ref pointer action on the bound Hyprland instance may aim at a
//! client. The compositor's stack proves only which toplevel takes a point.
//! A native Wayland client then routes the point inside its own surfaces,
//! and neither the stack nor the accessibility descent shows what takes it
//! there: an in-window overlay or a same-client popup took ref clicks while
//! both reported the target (probes/linux/FINDINGS.md L42-4, L42-5). So a
//! native Wayland target refuses before any event. An XWayland target keeps
//! the stack proof, which these rows did not measure.

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};
use serde_json::{Value, json};

use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::clients::client_parse::parse_address;
use crate::system::hyprland::ipc::bound_request::bound_request;
use crate::system::hyprland::receiver::receiver_refusal::ReceiverRefusal;

const XWAYLAND_EVIDENCE: &str = "client_xwayland";

/// Refuses a ref pointer action whose target window is not an XWayland
/// client, read from the bound instance's `j/clients`.
pub(crate) fn refuse_native_receiver(
    binding: &HyprlandBinding,
    address: u64,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    check_native_read(bound_request(binding, "j/clients", deadline), address)
}

/// The same check over the `j/clients` read itself. A failed read refuses as
/// missing evidence, or as timed out evidence when it ran out of time.
pub(crate) fn check_native_read(
    read: Result<Vec<u8>, AdapterError>,
    address: u64,
) -> Result<(), AdapterError> {
    check_native_receiver(&read?, address)
}

/// The same check over a `j/clients` reply. A reply that does not name the
/// target with an `xwayland` flag refuses as missing evidence.
pub(crate) fn check_native_receiver(clients: &[u8], address: u64) -> Result<(), AdapterError> {
    match xwayland_flag(clients, address) {
        Some(true) => Ok(()),
        Some(false) => Err(native_refusal()),
        None => Err(ReceiverRefusal::EvidenceMissing {
            check: XWAYLAND_EVIDENCE,
        }
        .to_error()),
    }
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

#[cfg(test)]
#[path = "native_receiver_tests.rs"]
mod tests;
