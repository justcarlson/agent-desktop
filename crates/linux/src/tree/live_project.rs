use agent_desktop_core::{
    AdapterError, ElementState, ErrorCode, LiveElement, LiveIdentity, LocatorEvidence, Rect, state,
};

use super::frame_geometry::pointer_bounds;
use super::offscreen::{OffscreenInput, offscreen};
use super::placement::Placement;
use super::state_bits::{ENABLED, SENSITIVE, SHOWING, StateBits, VISIBLE};
use super::{Extents, Field, ObservedRead};

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

fn has_position(read: &ObservedRead) -> bool {
    matches!(&read.node.extents, Field::Present(extents)
        if extents.x != i32::MIN && extents.y != i32::MIN)
}

fn is_sentinel(read: &ObservedRead) -> bool {
    matches!(&read.node.extents, Field::Present(_)) && !has_position(read)
}

fn state_bits(read: &ObservedRead) -> Option<(bool, bool)> {
    read.node.traits.states.present().map(|words| {
        let bits = StateBits::new(words);
        (bits.has(VISIBLE), bits.has(SHOWING))
    })
}

/// Whether the one node read already settles offscreen without looking at the
/// window: the state set failed or shows the node hidden, or it has the GTK 3
/// shape of a row scrolled out of its viewport (`VISIBLE` without `SHOWING`
/// at the sentinel position, L6-9).
pub(crate) fn settled_by_read(read: &ObservedRead) -> bool {
    match state_bits(read) {
        None | Some((false, false)) => true,
        Some((visible, showing)) => visible && !showing && is_sentinel(read),
    }
}

/// `Some(true)` only on positive evidence, `Some(false)` only when the node
/// provably intersects its frame and scroll container in a window that is not
/// minimized, `None` when anything needed was not read.
pub(crate) fn derived_offscreen(read: &ObservedRead, placement: &Placement) -> Option<bool> {
    let (visible, showing) = state_bits(read)?;
    if !visible && !showing {
        return None;
    }
    let minimized = if placement.frame_iconified || placement.x11.is_some_and(|x11| x11.hidden) {
        Some(true)
    } else {
        placement.x11.map(|_| false)
    };
    let verdict = offscreen(OffscreenInput {
        node: placement.node,
        frame: placement.frame,
        container: placement.container,
        visible,
        showing,
        sentinel: is_sentinel(read),
        minimized,
    });
    let container_unread =
        read.node.element.scroll_container().is_some() && placement.container.is_none();
    if container_unread && verdict == Some(false) {
        None
    } else {
        verdict
    }
}

/// The node's rectangle in the frame a pointer is aimed in, for a toolkit
/// whose own screen extents cannot be trusted: the toolkit's rule applied to
/// the screen extents, the window extents and the client origin. `None` when
/// nothing was read or the position cannot be proved.
pub(crate) fn pointer_rect(read: &ObservedRead, placement: &Placement) -> Option<Rect> {
    let Field::Present(screen) = &read.node.extents else {
        return None;
    };
    let window: Extents = placement.node?;
    let origin = placement.x11.map(|x11| x11.client_origin);
    pointer_bounds(&placement.toolkit, *screen, window, origin).map(Rect::from)
}

/// The state projection. `hidden` is a live-only token. `offscreen` follows
/// the placement: the token and `Some(true)` together on positive evidence.
pub(crate) fn live_state(
    read: &ObservedRead,
    evidence: &LocatorEvidence,
    placement: &Placement,
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
    let off = derived_offscreen(read, placement);
    if off == Some(true) && !states.iter().any(|token| token == state::OFFSCREEN) {
        states.push(state::OFFSCREEN.to_owned());
    }
    Ok(ElementState {
        role,
        states,
        value: live_value(evidence)?,
        enabled: enabled(read),
        hidden: None,
        offscreen: off,
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
/// A toolkit whose screen extents are global keeps them; any other takes the
/// provable pointer rectangle and falls back to the extents as read.
pub(crate) fn live_element(
    read: &ObservedRead,
    evidence: &LocatorEvidence,
    placement: &Placement,
    global: bool,
) -> Result<LiveElement, AdapterError> {
    let bounds = if global {
        live_bounds(read)
    } else {
        pointer_rect(read, placement).or_else(|| live_bounds(read))
    };
    Ok(LiveElement {
        identity: LiveIdentity {
            name: evidence.name.clone(),
            description: evidence.description.clone(),
            identifiers: evidence.identifiers.clone(),
        },
        state: live_state(read, evidence, placement)?,
        states_complete: true,
        bounds,
        available_actions: live_actions(evidence)?,
    })
}

#[cfg(test)]
#[path = "live_project_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "live_placement_tests.rs"]
mod placement_tests;
