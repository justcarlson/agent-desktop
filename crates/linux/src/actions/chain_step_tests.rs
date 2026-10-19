use agent_desktop_core::{ActionStepOutcome, StepMechanism};

use super::*;

#[test]
fn a_delivered_step_is_a_semantic_succeeded_step_with_the_given_verdict() {
    for (verified, expected) in [
        (Some(true), Some(true)),
        (Some(false), Some(false)),
        (None, None),
    ] {
        let step = semantic_step("Action.DoAction", verified);
        assert!(matches!(step.outcome, ActionStepOutcome::Succeeded));
        assert_eq!(step.mechanism(), Some(StepMechanism::SemanticApi));
        assert_eq!(step.verified(), expected);
        assert_eq!(step.label(), "Action.DoAction");
    }
}

#[test]
fn a_skipped_step_claims_no_verification_and_a_satisfied_one_claims_true() {
    let skipped = skipped_step("Action.DoAction");
    assert!(matches!(skipped.outcome, ActionStepOutcome::Skipped));
    assert_eq!(skipped.verified(), None);
    assert_eq!(skipped.mechanism(), Some(StepMechanism::SemanticApi));
    assert_eq!(satisfied_step("AlreadyInState").verified(), Some(true));
}

#[test]
fn an_observation_maps_to_the_verified_field() {
    assert_eq!(verified_of(Observed::Matched), Some(true));
    assert_eq!(verified_of(Observed::Unchanged), Some(false));
    assert_eq!(verified_of(Observed::Unread), None);
}
