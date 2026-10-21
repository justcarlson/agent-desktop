use std::time::Duration;

use agent_desktop_core::{Action, ActionResult, AdapterError, InteractionPolicy};
use serde_json::json;

use super::click::{POLL_STEP, deliver, invoke_verb, names_of, poll_until, run_single_rung};
use super::context::Target;
use crate::tree::action_names::verbs_for;
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

pub(super) fn corroborated(first: Option<bool>, second: Option<bool>) -> Option<bool> {
    match first {
        Some(true) => second,
        other => other,
    }
}

pub(crate) fn focus(
    target: &Target<'_>,
    policy: InteractionPolicy,
) -> Result<ActionResult, AdapterError> {
    focus_policy_gate(policy)?;
    let action = Action::SetFocus;
    let view = target.view()?;
    let names = names_of(&view)?;
    let verbs = verbs_for(&action);
    let run = || {
        deliver(invoke_verb(target, &action, names, verbs), || {
            let read = || target.view().ok().and_then(|after| after.state(FOCUSED));
            let first = poll_until(target.deadline, FOCUS_SETTLE, POLL_STEP, read, |focused| {
                *focused == Some(true)
            });
            if first == Some(true) {
                std::thread::sleep(POLL_STEP);
            }
            corroborated(first, read())
        })
    };
    run_single_rung(action.name(), target.deadline, run)
}

#[cfg(test)]
#[path = "focus_tests.rs"]
mod tests;
