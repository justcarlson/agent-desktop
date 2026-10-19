use agent_desktop_core::{AdapterError, DeliveryDisposition, RetryDisposition};

use super::ReleaseState;

const SUGGESTION: &str = "inspect before retrying";

#[test]
fn nothing_committed_reports_not_delivered_and_stays_safe_to_retry() {
    let state = ReleaseState::default();

    let error = state.enrich_error(AdapterError::internal("before any event"), SUGGESTION);

    assert_eq!(
        error.disposition.delivery(),
        DeliveryDisposition::NotDelivered
    );
    assert_eq!(error.disposition.retry(), RetryDisposition::Safe);
    assert!(error.details.is_none());
}

#[test]
fn three_delivered_events_are_reported_with_an_unacknowledged_emergency_release() {
    let mut state = ReleaseState::default();
    state.arm();
    for _ in 0..3 {
        state.mark_delivered();
    }

    let error = state.enrich_error(AdapterError::timeout("deadline"), SUGGESTION);

    assert_eq!(
        error.disposition.delivery(),
        DeliveryDisposition::DeliveredUnverified
    );
    assert_eq!(error.disposition.retry(), RetryDisposition::Unsafe);
    let details = error.details.expect("delivered input reports evidence");
    assert_eq!(details["delivered_events"], 3);
    assert_eq!(details["emergency_release_posted"], true);
    assert_eq!(details["emergency_release_acknowledged"], false);
    assert_eq!(error.suggestion.as_deref(), Some(SUGGESTION));
}

#[test]
fn a_disarmed_state_reports_delivery_without_an_emergency_release() {
    let mut state = ReleaseState::default();
    state.arm();
    state.mark_delivered();
    state.disarm();

    let error = state.enrich_error(AdapterError::timeout("after a clean release"), SUGGESTION);

    let details = error.details.expect("delivered input reports evidence");
    assert_eq!(details["delivered_events"], 1);
    assert!(details.get("emergency_release_posted").is_none());
    assert!(!state.should_release());
}

#[test]
fn existing_details_survive_the_enrichment() {
    let mut state = ReleaseState::resumed(2, true);
    state.mark_delivered();

    let error = state.enrich_error(
        AdapterError::timeout("deadline").with_details(serde_json::json!({ "kind": "deadline" })),
        SUGGESTION,
    );

    let details = error.details.expect("details");
    assert_eq!(details["kind"], "deadline");
    assert_eq!(details["delivered_events"], 3);
}
