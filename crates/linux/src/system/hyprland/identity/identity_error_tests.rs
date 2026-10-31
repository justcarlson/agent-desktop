use agent_desktop_core::{DeliverySemantics, ErrorCode};

use super::*;

#[test]
fn every_identity_refusal_is_not_delivered_and_safe_to_retry() {
    for refusal in [
        IdentityRefusal::ProcessChanged,
        IdentityRefusal::OtherCompositor,
        IdentityRefusal::ToplevelGone,
        IdentityRefusal::NoCandidate,
        IdentityRefusal::Ambiguous(vec![(1, "1".into()), (2, "2".into())]),
    ] {
        assert_eq!(
            identity_error(refusal).disposition,
            DeliverySemantics::not_delivered()
        );
    }
}

#[test]
fn a_changed_process_is_a_stale_window_and_a_vanished_toplevel_is_not_found() {
    assert_eq!(
        identity_error(IdentityRefusal::ProcessChanged).code,
        ErrorCode::StaleRef
    );
    assert_eq!(
        identity_error(IdentityRefusal::OtherCompositor).code,
        ErrorCode::WindowNotFound
    );
    assert_eq!(
        identity_error(IdentityRefusal::ToplevelGone).code,
        ErrorCode::WindowNotFound
    );
}

#[test]
fn no_candidate_and_twins_name_their_cause_and_twins_name_every_candidate() {
    let none = identity_error(IdentityRefusal::NoCandidate);
    let twins = identity_error(IdentityRefusal::Ambiguous(vec![
        (0x10, "1".into()),
        (0x2a, "special:scratch".into()),
    ]));

    assert_eq!(none.code, ErrorCode::ActionFailed);
    assert_eq!(none.details.unwrap()["cause"], "window_not_mapped");
    assert_eq!(twins.code, ErrorCode::ActionFailed);
    let details = twins.details.unwrap();
    assert_eq!(details["cause"], "ambiguous_window");
    assert_eq!(details["physical_delivery_started"], false);
    assert_eq!(
        details["candidates"],
        serde_json::json!([
            {"address": "0x10", "workspace": "1"},
            {"address": "0x2a", "workspace": "special:scratch"},
        ])
    );
}
