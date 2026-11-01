use agent_desktop_core::{Point, Rect};

use super::overlay_filter::OverlayFilter;
use super::receiver_refusal::ReceiverRefusal;
use super::stack::ReceiverStack;
use super::stack_client::StackClient;

const ABOVE_WINDOWS: u8 = 2;

/// Refuses unless `point` lies inside the shown toplevel `target` with no
/// top or overlay layer and no floating, pinned or fullscreen window over
/// it, counting a window only when it draws above the target. A layer counts
/// as a cover unless `filter` accepts it as this tool's
/// own cursor overlay.
pub(crate) fn check_receiver(
    stack: &ReceiverStack,
    target: u64,
    point: &Point,
    filter: &dyn OverlayFilter,
) -> Result<(), ReceiverRefusal> {
    let (target_rank, client) = stack
        .clients
        .iter()
        .enumerate()
        .find(|(_, client)| client.address == target)
        .ok_or(ReceiverRefusal::EvidenceMissing {
            check: "target_client",
        })?;
    if !client.shown || !(client.pinned || stack.desktop.in_view(&client.workspace)) {
        return Err(ReceiverRefusal::TargetNotShown);
    }
    if !holds(&client.rect, point) {
        return Err(ReceiverRefusal::PointOutsideTarget);
    }
    if let Some(layer) = stack.layers.iter().find(|layer| {
        layer.level >= ABOVE_WINDOWS && holds(&layer.rect, point) && !filter.is_own_cursor(layer)
    }) {
        return Err(ReceiverRefusal::LayerCover {
            namespace: layer.namespace.clone(),
            level: layer.level,
        });
    }
    if let Some((_, cover)) = stack.clients.iter().enumerate().find(|(rank, other)| {
        other.address != target
            && above((*rank, other), (target_rank, client))
            && other.shown
            && (other.floating || other.pinned || other.fullscreen)
            && (other.pinned || stack.desktop.in_view(&other.workspace))
            && holds(&other.rect, point)
    }) {
        return Err(ReceiverRefusal::WindowCover {
            address: cover.address,
        });
    }
    Ok(())
}

/// Whether `other` draws above `target`. Hyprland lists `j/clients` in its
/// stacking order, bottom first, and draws every floating, pinned or
/// fullscreen window above the tiled ones and every pinned window above the
/// rest; within one of those groups the later window is on top.
fn above(other: (usize, &StackClient), target: (usize, &StackClient)) -> bool {
    let group = |client: &StackClient| match (client.pinned, client.floating || client.fullscreen) {
        (true, _) => 2,
        (false, true) => 1,
        (false, false) => 0,
    };
    (group(other.1), other.0) > (group(target.1), target.0)
}

pub(crate) fn holds(rect: &Rect, point: &Point) -> bool {
    point.x >= rect.x
        && point.x < rect.x + rect.width
        && point.y >= rect.y
        && point.y < rect.y + rect.height
}

#[cfg(test)]
#[path = "check_receiver_tests.rs"]
mod tests;
