use std::time::Duration;

use agent_desktop_core::{Action, ActionResult, AdapterError, InteractionPolicy};
use serde_json::json;

use super::chain::DeliveryOutcome;
use super::click::{POLL_STEP, deliver, invoke_verb, names_of, poll_until, run_single_rung};
use super::context::Target;
use super::mutation::WriteReply;
use crate::tree::action_names::verbs_for;
use crate::tree::action_read::read_node_view;
use crate::tree::state_bits::FOCUSED;

const FOCUS_SETTLE: Duration = Duration::from_millis(500);

pub(super) fn focus_policy_gate(policy: InteractionPolicy) -> Result<(), AdapterError> {
    if policy.allow_focus_steal {
        return Ok(());
    }
    Err(AdapterError::policy_denied_for_policy(
        "Focusing an element moves the keyboard focus of the application",
        policy,
    )
    .with_details(json!({ "foreground_effect": true })))
}

pub(super) fn focus_judged_for(
    policy: InteractionPolicy,
    invoke: impl FnOnce() -> Result<WriteReply, AdapterError>,
    focused_after: impl FnOnce() -> Option<bool>,
) -> Result<DeliveryOutcome, AdapterError> {
    focus_policy_gate(policy)?;
    deliver(invoke(), focused_after)
}

pub(crate) fn focus(
    target: &Target<'_>,
    policy: InteractionPolicy,
) -> Result<ActionResult, AdapterError> {
    focus_policy_gate(policy)?;
    let action = Action::SetFocus;
    let view = read_node_view(target.bus, target.handle, target.deadline)?;
    let names = names_of(&view)?;
    let verbs = verbs_for(&action);
    let run = || {
        focus_judged_for(
            policy,
            || invoke_verb(target, &action, names, verbs),
            || {
                poll_until(
                    target.deadline,
                    FOCUS_SETTLE,
                    POLL_STEP,
                    || {
                        read_node_view(target.bus, target.handle, target.deadline)
                            .ok()
                            .and_then(|after| after.state(FOCUSED))
                    },
                    |focused| *focused == Some(true),
                )
            },
        )
    };
    run_single_rung(action.name(), target.deadline, run)
}

#[cfg(test)]
#[path = "focus_tests.rs"]
mod tests;
