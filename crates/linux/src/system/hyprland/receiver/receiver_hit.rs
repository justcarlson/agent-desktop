//! A hit test answered from the bound instance's stack: whether a point
//! reaches the target's toplevel or which window or layer surface holds it.

use agent_desktop_core::{AdapterError, Deadline, HitTestResult, Point};

use super::check_receiver::{check_receiver, holds};
use super::deny_all_overlays::DenyAllOverlays;
use super::receiver_read::read_stack;
use super::receiver_refusal::ReceiverRefusal;
use super::stack::ReceiverStack;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;

/// What `point` reaches in `stack` for the toplevel at `target`. A cover
/// names the window or layer surface that holds the point; missing evidence,
/// a target that is not shown, or a point outside it is unknown.
pub(crate) fn hit_of(stack: &ReceiverStack, target: u64, point: &Point) -> HitTestResult {
    match check_receiver(stack, target, point, &DenyAllOverlays) {
        Ok(()) => HitTestResult::ReachesTarget,
        Err(ReceiverRefusal::WindowCover { address }) => HitTestResult::InterceptedBy {
            role: Some("window".to_owned()),
            name: None,
            bounds: stack
                .clients
                .iter()
                .find(|client| client.address == address)
                .map(|client| client.rect),
        },
        Err(ReceiverRefusal::LayerCover { namespace, level }) => HitTestResult::InterceptedBy {
            role: Some("layer".to_owned()),
            bounds: stack
                .layers
                .iter()
                .find(|layer| {
                    layer.namespace == namespace
                        && layer.level == level
                        && holds(&layer.rect, point)
                })
                .map(|layer| layer.rect),
            name: Some(namespace),
        },
        Err(_) => HitTestResult::Unknown,
    }
}

/// [`hit_of`] on a fresh read of the bound instance's stack. A read that
/// runs out of time is a timeout; any other failed read is unknown.
pub(crate) fn read_hit(
    binding: &HyprlandBinding,
    target: u64,
    point: &Point,
    deadline: Deadline,
) -> Result<HitTestResult, AdapterError> {
    match read_stack(binding, deadline) {
        Ok(stack) => Ok(hit_of(&stack, target, point)),
        Err(refusal @ ReceiverRefusal::EvidenceTimedOut { .. }) => Err(refusal.to_error()),
        Err(_) => Ok(HitTestResult::Unknown),
    }
}

#[cfg(test)]
#[path = "receiver_hit_tests.rs"]
mod tests;
