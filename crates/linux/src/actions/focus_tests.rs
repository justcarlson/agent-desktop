use std::cell::Cell;

use agent_desktop_core::{DeliverySemantics, ErrorCode};

use super::*;
use crate::actions::chain::Observed;

fn judged(
    policy: InteractionPolicy,
    reply: Result<WriteReply, AdapterError>,
    focused: Option<bool>,
) -> (Result<DeliveryOutcome, AdapterError>, u32) {
    let writes = Cell::new(0);
    let outcome = focus_judged_for(
        policy,
        || {
            writes.set(writes.get() + 1);
            reply
        },
        || focused,
    );
    (outcome, writes.get())
}

#[test]
fn headless_focus_is_denied_with_a_foreground_effect_and_no_write() {
    let (outcome, writes) = judged(
        InteractionPolicy::headless(),
        Ok(WriteReply::Done),
        Some(true),
    );
    let error = outcome.expect_err("denied");
    assert_eq!(error.code, ErrorCode::PolicyDenied);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert_eq!(
        error.details.expect("details")["foreground_effect"],
        serde_json::json!(true)
    );
    assert_eq!(writes, 0);
}

#[test]
fn cursor_only_policy_does_not_allow_focus() {
    let policy = InteractionPolicy {
        allow_focus_steal: false,
        allow_cursor_move: true,
    };
    let (outcome, writes) = judged(policy, Ok(WriteReply::Done), Some(true));
    assert_eq!(outcome.expect_err("denied").code, ErrorCode::PolicyDenied);
    assert_eq!(writes, 0);
}

#[test]
fn headed_focus_with_a_later_focused_state_is_matched() {
    let (outcome, writes) = judged(
        InteractionPolicy::headed(),
        Ok(WriteReply::Done),
        Some(true),
    );
    assert_eq!(
        outcome.expect("delivered"),
        DeliveryOutcome::Delivered(Observed::Matched)
    );
    assert_eq!(writes, 1);
}

#[test]
fn focus_fallback_policy_may_focus() {
    let (outcome, _) = judged(
        InteractionPolicy::focus_fallback(),
        Ok(WriteReply::Done),
        Some(true),
    );
    assert!(outcome.is_ok());
}

#[test]
fn a_state_that_stays_unfocused_is_unchanged() {
    let (outcome, _) = judged(
        InteractionPolicy::headed(),
        Ok(WriteReply::Done),
        Some(false),
    );
    assert_eq!(
        outcome.expect("delivered"),
        DeliveryOutcome::Delivered(Observed::Unchanged)
    );
}

#[test]
fn an_unread_focus_state_is_unread() {
    let (outcome, _) = judged(InteractionPolicy::headed(), Ok(WriteReply::Done), None);
    assert_eq!(
        outcome.expect("delivered"),
        DeliveryOutcome::Delivered(Observed::Unread)
    );
}
