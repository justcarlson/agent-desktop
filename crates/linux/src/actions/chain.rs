use agent_desktop_core::{
    ActionResult, ActionStep, AdapterError, Deadline, DeliverySemantics, ErrorCode,
};
use serde_json::json;

use super::chain_step::{satisfied_step, semantic_step, skipped_step, verified_of};
use crate::system::ensure_budget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Observed {
    Matched,
    Unchanged,
    Unread,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeliveryOutcome {
    NotDelivered,
    Satisfied,
    Delivered(Observed),
}

pub(crate) struct ChainDef {
    pub(crate) action: &'static str,
    pub(crate) suggestion: &'static str,
}

pub(crate) struct Rung<'a> {
    pub(crate) label: &'static str,
    pub(crate) run: Box<dyn FnMut() -> Result<DeliveryOutcome, AdapterError> + 'a>,
}

fn exhausted(def: &ChainDef, steps: &[ActionStep]) -> AdapterError {
    let tried: Vec<&str> = steps.iter().map(ActionStep::label).collect();
    AdapterError::new(ErrorCode::ActionFailed, "All chain steps exhausted")
        .with_suggestion(def.suggestion)
        .with_details(json!({ "steps_tried": tried }))
        .with_disposition(DeliverySemantics::not_delivered())
}

fn expired(error: AdapterError) -> AdapterError {
    error.with_disposition(DeliverySemantics::not_delivered())
}

pub(crate) fn execute_chain(
    def: &ChainDef,
    rungs: &mut [Rung<'_>],
    deadline: Deadline,
) -> Result<ActionResult, AdapterError> {
    let mut steps = Vec::new();
    for rung in rungs.iter_mut() {
        ensure_budget(deadline).map_err(expired)?;
        match (rung.run)()? {
            DeliveryOutcome::NotDelivered => steps.push(skipped_step(rung.label)),
            DeliveryOutcome::Satisfied => {
                steps.push(satisfied_step(rung.label));
                return Ok(ActionResult::satisfied_without_delivery(def.action).with_steps(steps));
            }
            DeliveryOutcome::Delivered(observed) => {
                let verified = verified_of(observed);
                steps.push(semantic_step(rung.label, verified));
                let result = ActionResult::delivered_unverified(def.action).with_steps(steps);
                return Ok(match verified {
                    Some(true) => result.with_verified_delivery(),
                    _ => result,
                });
            }
        }
    }
    Err(exhausted(def, &steps))
}

#[cfg(test)]
#[path = "chain_tests.rs"]
mod tests;
