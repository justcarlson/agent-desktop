use super::*;

#[test]
fn a_held_edge_is_not_supported_and_names_what_was_refused() {
    let error = reject_standalone("key-down/key-up");
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    assert!(error.message.contains("key-down/key-up"));
}

#[test]
fn the_error_carries_all_three_details_and_the_not_delivered_disposition() {
    let error = reject_standalone("mouse-down/mouse-up");
    let details = error.details.clone().unwrap();
    assert_eq!(details["raw_input_emitted"], false);
    assert_eq!(details["requires_daemon_owned_transaction"], true);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
}
