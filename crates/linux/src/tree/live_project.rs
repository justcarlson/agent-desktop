use agent_desktop_core::{
    AdapterError, ElementState, ErrorCode, LiveElement, LiveIdentity, LocatorEvidence, Rect, state,
};

use super::state_bits::{ENABLED, SENSITIVE, SHOWING, StateBits, VISIBLE};
use super::{Field, ObservedRead};

/// The answer for a live read that left a field the projection needs unread.
/// It is retryable: the node answered, a field did not.
pub(crate) fn incomplete_live_evidence() -> AdapterError {
    AdapterError::new(
        ErrorCode::AppUnresponsive,
        "Live element evidence was incomplete",
    )
    .with_details(serde_json::json!({
        "kind": "live_element_evidence",
        "complete": false,
        "retryable": true,
    }))
}

/// Whether a state read succeeded and neither `VISIBLE` nor `SHOWING` is set.
/// `SHOWING` alone cannot decide it: on-screen Chromium and Gecko nodes report
/// `VISIBLE` without `SHOWING` (L3-27), and `SHOWING` is a strict subset of
/// `VISIBLE` (L3-7).
fn is_hidden(read: &ObservedRead) -> bool {
    read.node.traits.states.present().is_some_and(|words| {
        let bits = StateBits::new(words);
        !bits.has(VISIBLE) && !bits.has(SHOWING)
    })
}

/// `ENABLED` or `SENSITIVE` (GTK4 sets only the second) makes a control
/// enabled; a successful state read with neither makes it disabled.
fn enabled(read: &ObservedRead) -> Option<bool> {
    read.node.traits.states.present().map(|words| {
        let bits = StateBits::new(words);
        bits.has(ENABLED) || bits.has(SENSITIVE)
    })
}

fn secure(evidence: &LocatorEvidence) -> bool {
    evidence
        .states
        .known()
        .is_some_and(|tokens| tokens.iter().any(|token| token == state::SECURE))
}

/// The node's value, withheld from a secure field whatever was read.
pub(crate) fn live_value(evidence: &LocatorEvidence) -> Result<Option<String>, AdapterError> {
    if secure(evidence) {
        return Ok(None);
    }
    if evidence.value.is_unknown() {
        return Err(incomplete_live_evidence());
    }
    Ok(evidence.value.known().cloned())
}

/// The state projection. `hidden` is a live-only token; `offscreen` is never
/// produced because the state set cannot decide it.
pub(crate) fn live_state(
    read: &ObservedRead,
    evidence: &LocatorEvidence,
) -> Result<ElementState, AdapterError> {
    let role = evidence
        .role
        .known()
        .cloned()
        .ok_or_else(incomplete_live_evidence)?;
    let mut states = evidence
        .states
        .known()
        .cloned()
        .ok_or_else(incomplete_live_evidence)?;
    if is_hidden(read) {
        states.push(state::HIDDEN.to_owned());
    }
    Ok(ElementState {
        role,
        states,
        value: live_value(evidence)?,
        enabled: enabled(read),
        hidden: None,
        offscreen: None,
    })
}

pub(crate) fn live_actions(evidence: &LocatorEvidence) -> Result<Vec<String>, AdapterError> {
    match &evidence.ref_evidence.available_actions {
        agent_desktop_core::LocatorField::Known(actions) => Ok(actions.clone()),
        agent_desktop_core::LocatorField::Absent => Ok(Vec::new()),
        agent_desktop_core::LocatorField::Unknown => Err(incomplete_live_evidence()),
    }
}

/// The node's extents as the toolkit reports them. Zero and `-1` extents are
/// values. The position `INT32_MIN` a toolkit gives a node off every screen is
/// no bounds, and so is a node without a `Component` interface or one whose
/// extents could not be read.
pub(crate) fn live_bounds(read: &ObservedRead) -> Option<Rect> {
    match &read.node.extents {
        Field::Present(extents) if extents.x != i32::MIN && extents.y != i32::MIN => {
            Some(Rect::from(*extents))
        }
        Field::Present(_) | Field::Absent | Field::Failed(_) => None,
    }
}

/// Identity, state, bounds and actions of one node, all from the one read.
pub(crate) fn live_element(
    read: &ObservedRead,
    evidence: &LocatorEvidence,
) -> Result<LiveElement, AdapterError> {
    Ok(LiveElement {
        identity: LiveIdentity {
            name: evidence.name.clone(),
            description: evidence.description.clone(),
            identifiers: evidence.identifiers.clone(),
        },
        state: live_state(read, evidence)?,
        states_complete: true,
        bounds: live_bounds(read),
        available_actions: live_actions(evidence)?,
    })
}

#[cfg(test)]
#[path = "live_project_tests.rs"]
mod tests;
