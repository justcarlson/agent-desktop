use std::cell::Cell;

use agent_desktop_core::{DeliverySemantics, ErrorCode};

use super::*;
use crate::actions::chain::DeliveryOutcome;
use crate::actions::mutation::WriteReply;

fn judged(
    policy: InteractionPolicy,
    reply: Result<WriteReply, AdapterError>,
    focused: Option<bool>,
) -> (Result<DeliveryOutcome, AdapterError>, u32) {
    let writes = Cell::new(0);
    let outcome = focus_policy_gate(policy).and_then(|()| {
        deliver(
            {
                writes.set(writes.get() + 1);
                reply
            },
            || focused,
        )
    });
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
fn focus_that_drops_on_the_second_read_is_not_verified() {
    assert_eq!(corroborated(Some(true), Some(false)), Some(false));
}

#[test]
fn focus_held_across_two_reads_is_verified() {
    assert_eq!(corroborated(Some(true), Some(true)), Some(true));
}

#[test]
fn focus_not_seen_on_the_first_read_is_not_re_read_into_a_match() {
    assert_eq!(corroborated(Some(false), Some(true)), Some(false));
    assert_eq!(corroborated(None, Some(true)), None);
}

#[test]
fn a_failed_second_read_leaves_focus_unproven() {
    assert_eq!(corroborated(Some(true), None), None);
}
