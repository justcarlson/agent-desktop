use agent_desktop_core::{AdapterError, Deadline, HitTestResult, Point};

use super::light::{extents_of, is_placed};
use crate::system::{SharedBus, list_displays, within};
use crate::tree::display_bounds::off_every_display;
use crate::tree::{AtspiElement, Extents};

fn contains(extents: Extents, point: &Point) -> bool {
    let (x, y) = (point.x, point.y);
    x >= f64::from(extents.x)
        && x < f64::from(extents.x) + f64::from(extents.width)
        && y >= f64::from(extents.y)
        && y < f64::from(extents.y) + f64::from(extents.height)
}

/// A point query on the shell returns a coarse container and cannot confirm
/// a target, so the verdict rests on the target's own extents: reached when
/// the point lies inside them and they are on a display, unknown otherwise,
/// and never intercepted.
pub(crate) fn decide(
    extents: Option<Extents>,
    point: &Point,
    displays: Option<&[Extents]>,
) -> HitTestResult {
    let Some(extents) = extents.filter(|extents| is_placed(*extents)) else {
        return HitTestResult::Unknown;
    };
    let Some(displays) = displays else {
        return HitTestResult::Unknown;
    };
    if off_every_display(Some(extents), Some(displays)) || displays.is_empty() {
        return HitTestResult::Unknown;
    }
    if contains(extents, point) {
        HitTestResult::ReachesTarget
    } else {
        HitTestResult::Unknown
    }
}

pub(crate) fn display_extents(deadline: Deadline) -> Option<Vec<Extents>> {
    let displays = list_displays(deadline).ok()?;
    Some(
        displays
            .iter()
            .map(|display| Extents {
                x: display.bounds.x as i32,
                y: display.bounds.y as i32,
                width: display.bounds.width as i32,
                height: display.bounds.height as i32,
            })
            .collect(),
    )
}

pub(crate) fn hit_test(
    bus: &SharedBus,
    target: &AtspiElement,
    point: Point,
    deadline: Deadline,
) -> Result<HitTestResult, AdapterError> {
    let connection = bus.get(deadline)?.inner().clone();
    let extents = within(deadline, async {
        Ok(extents_of(&connection, target).await)
    })?;
    let displays = display_extents(deadline);
    Ok(decide(extents, &point, displays.as_deref()))
}

#[cfg(test)]
#[path = "hit_tests.rs"]
mod tests;
