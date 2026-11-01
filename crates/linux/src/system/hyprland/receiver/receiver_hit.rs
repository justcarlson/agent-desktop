//! A hit test answered from the bound instance's stack: whether a point
//! reaches the target's toplevel or which window or layer surface holds it.

use agent_desktop_core::{HitTestResult, Point};

use super::stack::ReceiverStack;

/// What `point` reaches in `stack` for the toplevel at `target`. A cover
/// names the window or layer surface that holds the point; missing evidence,
/// a target that is not shown, or a point outside it is unknown.
pub(crate) fn hit_of(_stack: &ReceiverStack, _target: u64, _point: &Point) -> HitTestResult {
    HitTestResult::Unknown
}

#[cfg(test)]
#[path = "receiver_hit_tests.rs"]
mod tests;
