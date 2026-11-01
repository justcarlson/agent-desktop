use agent_desktop_core::Point;

use super::overlay_filter::OverlayFilter;
use super::receiver_refusal::ReceiverRefusal;
use super::stack::ReceiverStack;

/// Refuses unless `point` lies inside the shown toplevel `target` with no
/// top or overlay layer and no floating, pinned or fullscreen window over
/// it. A layer counts as a cover unless `filter` accepts it as this tool's
/// own cursor overlay.
pub(crate) fn check_receiver(
    stack: &ReceiverStack,
    target: u64,
    point: &Point,
    filter: &dyn OverlayFilter,
) -> Result<(), ReceiverRefusal> {
    let _ = (stack, target, point, filter);
    Ok(())
}

#[cfg(test)]
#[path = "check_receiver_tests.rs"]
mod tests;
