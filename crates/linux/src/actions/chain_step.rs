use agent_desktop_core::{ActionStep, StepMechanism};

#[expect(dead_code, reason = "no caller yet")]
pub(crate) fn semantic_step(label: &'static str, verified: Option<bool>) -> ActionStep {
    let step = ActionStep::succeeded(label).with_mechanism(StepMechanism::SemanticApi);
    match verified {
        Some(verified) => step.with_verified(verified),
        None => step,
    }
}

#[cfg(test)]
#[path = "chain_step_tests.rs"]
mod tests;
