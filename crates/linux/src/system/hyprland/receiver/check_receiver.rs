use agent_desktop_core::{Point, Rect};

use super::overlay_filter::OverlayFilter;
use super::receiver_refusal::ReceiverRefusal;
use super::stack::ReceiverStack;
use super::stack_client::StackClient;
use super::stacking::Stacking;

const ABOVE_WINDOWS: u8 = 2;

/// Refuses unless `point` lies inside the shown toplevel `target` with no
/// top or overlay layer and no floating, pinned or fullscreen window over
/// it, counting a window only when it takes the pointer before the target.
/// A layer counts as a cover unless `filter` accepts it as this tool's own
/// cursor overlay.
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
    if !client.shown || !(client.stacking.pinned || stack.desktop.in_view(&client.workspace)) {
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
    let mut unknown = false;
    for (rank, other) in stack.clients.iter().enumerate() {
        if !takes_point(stack, (rank, other), (target_rank, client), point) {
            continue;
        }
        match above((rank, other), (target_rank, client)) {
            Some(true) => {
                return Err(ReceiverRefusal::WindowCover {
                    address: other.address,
                });
            }
            Some(false) => {}
            None => unknown = true,
        }
    }
    if unknown {
        return Err(ReceiverRefusal::EvidenceMissing {
            check: "over_fullscreen",
        });
    }
    Ok(())
}

/// Whether `other` could take the pointer at `point` from `target`: a
/// shown floating, pinned or fullscreen window in view that holds the
/// point. A fullscreen window on the target's workspace takes every point
/// there, because Hyprland gives the pointer to it, or to nothing outside
/// a maximized one, before any window below it.
fn takes_point(
    stack: &ReceiverStack,
    other: (usize, &StackClient),
    target: (usize, &StackClient),
    point: &Point,
) -> bool {
    let (other, target) = (other.1, target.1);
    let stacking = other.stacking;
    other.address != target.address
        && other.shown
        && (stacking.floating || stacking.pinned || stacking.fullscreen)
        && (stacking.pinned || stack.desktop.in_view(&other.workspace))
        && (holds(&other.rect, point)
            || (stacking.fullscreen && other.workspace.name == target.workspace.name))
}

/// Whether `other` takes the pointer before `target`, or `None` when an
/// unreported over-fullscreen state decides it. `j/clients` lists windows
/// bottom first. Hyprland's pointer lookup takes pinned floating windows,
/// then floating windows allowed over fullscreen, then the fullscreen
/// window, then other floating windows, then tiled ones; within a rank the
/// later window is on top.
fn above(other: (usize, &StackClient), target: (usize, &StackClient)) -> Option<bool> {
    let (low, high) = rank(&other.1.stacking);
    let (target_low, target_high) = rank(&target.1.stacking);
    let surely = (low, other.0) > (target_high, target.0);
    let possibly = (high, other.0) > (target_low, target.0);
    (surely == possibly).then_some(surely)
}

fn rank(stacking: &Stacking) -> (u8, u8) {
    if stacking.pinned {
        return (4, 4);
    }
    match (
        stacking.fullscreen,
        stacking.floating,
        stacking.over_fullscreen,
    ) {
        (true, true, _) | (false, true, Some(true)) => (3, 3),
        (true, false, _) => (2, 2),
        (false, true, Some(false)) => (1, 1),
        (false, true, None) => (1, 3),
        (false, false, _) => (0, 0),
    }
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
