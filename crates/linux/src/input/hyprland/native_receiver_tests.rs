use agent_desktop_core::{DeliverySemantics, ErrorCode};

use super::*;

fn clients(xwayland: Option<bool>) -> Vec<u8> {
    let mut client = serde_json::json!({
        "address": "0x55e7761d2e70",
        "pid": 4242,
        "at": [0, 0],
        "size": [640, 480],
        "mapped": true,
        "hidden": false,
    });
    if let Some(flag) = xwayland {
        client["xwayland"] = serde_json::json!(flag);
    }
    let other = serde_json::json!({ "address": "0x1", "pid": 7, "xwayland": true });
    serde_json::to_vec(&serde_json::json!([other, client])).unwrap()
}

const TARGET: u64 = 0x55e7761d2e70;

#[test]
fn a_native_wayland_target_is_refused_before_any_event() {
    let error = check_native_receiver(&clients(Some(false)), TARGET).unwrap_err();
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    let details = error.details.as_ref().unwrap();
    assert_eq!(details["kind"], "receiver_evidence");
    assert_eq!(details["cause"], "native_wayland_client");
    assert_eq!(details["physical_delivery_started"], false);
    let suggestion = error.suggestion.as_deref().unwrap();
    assert!(suggestion.contains("mouse-click --xy"), "{suggestion}");
    assert!(suggestion.contains("receiver_proof: false"), "{suggestion}");
    assert!(suggestion.contains("without --headed"), "{suggestion}");
}

#[test]
fn a_native_wayland_refusal_never_falls_back_to_a_semantic_action() {
    let error = check_native_receiver(&clients(Some(false)), TARGET).unwrap_err();
    assert_ne!(error.code, ErrorCode::ActionNotSupported);
    let check = &error.details.as_ref().unwrap()["check"];
    assert_ne!(check, "geometry");
}

#[test]
fn an_xwayland_target_keeps_the_stack_proof() {
    assert!(check_native_receiver(&clients(Some(true)), TARGET).is_ok());
}

#[test]
fn a_target_without_an_xwayland_flag_is_refused_as_missing_evidence() {
    let error = check_native_receiver(&clients(None), TARGET).unwrap_err();
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    let details = error.details.as_ref().unwrap();
    assert_eq!(details["cause"], "evidence_missing");
    assert_eq!(details["evidence"], "client_xwayland");
}

#[test]
fn a_target_missing_from_the_client_list_is_refused() {
    let error = check_native_receiver(&clients(Some(true)), 0xdead).unwrap_err();
    assert_eq!(error.details.as_ref().unwrap()["cause"], "evidence_missing");
}

#[test]
fn an_unreadable_client_list_is_refused() {
    let error = check_native_receiver(b"not json", TARGET).unwrap_err();
    assert_eq!(error.details.as_ref().unwrap()["cause"], "evidence_missing");
}
