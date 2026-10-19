use super::visibility;
use crate::actionability::evidence::ActionabilityEvidence;
use crate::actionability::status::ActionabilityStatus;
use crate::{ElementState, Rect};

fn evidence(offscreen: Option<bool>, bounds: Option<Rect>) -> ActionabilityEvidence {
    ActionabilityEvidence {
        state: ElementState {
            role: "button".into(),
            states: vec![],
            value: None,
            enabled: Some(true),
            hidden: Some(false),
            offscreen,
        },
        states_complete: true,
        bounds,
        available_actions: vec![],
    }
}

#[test]
fn a_known_on_screen_node_passes_without_bounds_when_no_geometry_is_needed() {
    let check = visibility(&evidence(Some(false), None), false);
    assert_eq!(check.status, ActionabilityStatus::Pass);
}

#[test]
fn geometry_that_is_required_still_needs_bounds() {
    let check = visibility(&evidence(Some(false), None), true);
    assert_eq!(check.status, ActionabilityStatus::Unknown);
}

#[test]
fn an_unknown_offscreen_state_without_bounds_stays_unknown() {
    let check = visibility(&evidence(None, None), false);
    assert_eq!(check.status, ActionabilityStatus::Unknown);
}

#[test]
fn a_known_on_screen_node_with_zero_sized_bounds_still_fails() {
    let zero = Rect {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    };
    let check = visibility(&evidence(Some(false), Some(zero)), false);
    assert_ne!(check.status, ActionabilityStatus::Pass);
}
