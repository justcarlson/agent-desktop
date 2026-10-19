use super::*;

#[test]
fn a_combo_value_outranks_an_unselected_item() {
    assert_eq!(verification(Some(true), Some(false)), Some(true));
}

#[test]
fn a_combo_value_that_differs_outranks_a_selected_item() {
    assert_eq!(verification(Some(false), Some(true)), Some(false));
}

#[test]
fn a_combo_box_is_verified_even_when_the_item_state_is_unread() {
    assert_eq!(verification(Some(true), None), Some(true));
}

#[test]
fn without_a_combo_value_the_item_state_decides() {
    assert_eq!(verification(None, Some(true)), Some(true));
    assert_eq!(verification(None, Some(false)), Some(false));
}

#[test]
fn nothing_read_is_unverified() {
    assert_eq!(verification(None, None), None);
}

#[test]
fn a_matching_label_with_no_route_is_not_supported_and_not_delivered() {
    let error = no_route("Dark");
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    assert_eq!(
        error.disposition,
        agent_desktop_core::DeliverySemantics::not_delivered()
    );
}

#[test]
fn the_display_value_is_never_echoed_into_an_error() {
    let error = no_route("hunter2");
    assert!(!error.message.contains("hunter2"));
}

#[test]
fn a_select_child_that_answers_true_for_an_unselected_item_is_unchanged() {
    let outcome =
        deliver(Ok(WriteReply::Done), || verification(None, Some(false))).expect("delivered");
    assert_eq!(
        outcome,
        crate::actions::chain::DeliveryOutcome::Delivered(
            crate::actions::chain::Observed::Unchanged
        )
    );
}

#[test]
fn a_select_child_that_is_refused_is_not_delivered() {
    let outcome = deliver(Ok(WriteReply::Absent), || Some(true)).expect("answered");
    assert_eq!(
        outcome,
        crate::actions::chain::DeliveryOutcome::NotDelivered
    );
}

#[test]
fn a_select_whose_read_back_is_unchanged_is_a_write_ignored_failure() {
    use super::super::chain::{DeliveryOutcome, Observed};
    use super::unchanged_is_ignored;
    use agent_desktop_core::ErrorCode;
    let error = unchanged_is_ignored(DeliveryOutcome::Delivered(Observed::Unchanged))
        .expect_err("unchanged is ignored");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.details.expect("details")["cause"], "write_ignored");
    for kept in [
        DeliveryOutcome::Delivered(Observed::Matched),
        DeliveryOutcome::Delivered(Observed::Unread),
        DeliveryOutcome::NotDelivered,
    ] {
        assert!(unchanged_is_ignored(kept).is_ok());
    }
}
