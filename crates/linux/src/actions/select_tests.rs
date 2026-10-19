use super::*;
use crate::actions::mutation::WriteReply;
use crate::actions::select_verify::{combo_evidence, says_something, verification};

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
    let error = no_route();
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    assert_eq!(
        error.disposition,
        agent_desktop_core::DeliverySemantics::not_delivered()
    );
}

#[test]
fn no_error_text_in_select_reveals_the_length_of_the_value() {
    let source = include_str!("select.rs");
    assert!(!source.contains("character"));
    assert!(!source.contains("chars().count()"));
    assert!(!no_route().message.contains("character"));
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

#[test]
fn a_blank_or_object_replacement_text_says_nothing_about_the_combo_value() {
    assert!(!says_something(""));
    assert!(!says_something(" \u{fffc} "));
    assert!(says_something("Two"));
}

#[test]
fn a_combo_name_is_evidence_against_the_wanted_value_only_where_it_is_the_value() {
    assert_eq!(combo_evidence(None, Some("combo"), "beta", false), None);
    assert_eq!(combo_evidence(None, Some("sel"), "two", false), None);
    assert_eq!(
        combo_evidence(None, Some("combo"), "beta", true),
        Some(false)
    );
    assert_eq!(
        combo_evidence(None, Some("beta"), "beta", false),
        Some(true)
    );
    assert_eq!(
        combo_evidence(Some("alpha"), Some("beta"), "beta", false),
        Some(false)
    );
    assert_eq!(
        combo_evidence(Some("\u{fffc}"), Some(""), "beta", true),
        None
    );
}
