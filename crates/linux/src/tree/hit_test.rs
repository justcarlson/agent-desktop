use agent_desktop_core::{AdapterError, Deadline, HitTestResult, NativeHandle, Point};

use super::frame_geometry::{calibrate, to_pointer_frame, window_point};
use super::hit_cover::{cover_name, override_name};
use super::hit_descent::{Described, NodeSource, Probe, descend, locate};
use super::hit_guard::{admits, aimable, pixel};
use super::hit_occlusion::occlusion;
use super::hit_verdict::{Landing, Standing, Verdict, decide, needs_landing};
use super::toolkit_trust::{reports_global_position, trusts_descent, trusts_same_window_landing};
use super::{AtspiElement, Extents, atspi_element};
use crate::system::toolkit_id::ToolkitId;
use crate::system::x11::stacking_snapshot::StackingSnapshot;
use crate::system::x11::{FrameFacts, X11Client};
use crate::system::{SharedBus, shell};

mod live_world;

/// What the target's window and the target say about where they are.
#[derive(Debug, Clone)]
pub(crate) struct Facts {
    pub(crate) screen: Extents,
    pub(crate) window: Extents,
    pub(crate) frame: FrameFacts,
    pub(crate) minimized: bool,
    pub(crate) toolkit: ToolkitId,
}

/// The target's X11 client and the stacking order around it.
#[derive(Debug, Clone)]
pub(crate) struct Windowing {
    pub(crate) client: u32,
    pub(crate) snapshot: StackingSnapshot,
    pub(crate) clients: Vec<X11Client>,
    /// Popup WINDOW coordinates can start inside the owning frame. Keep that
    /// translation separate from the physical X11 rectangles used for covers.
    pub(crate) coordinate_origin: Option<(i32, i32)>,
}

/// Everything a hit test reads. The bus implements it; a test substitutes
/// in-memory nodes and records the calls.
pub(crate) trait HitWorld: NodeSource {
    /// The target's extents in both coordinate types and its window's facts,
    /// or `None` when any of them cannot be read.
    fn facts(
        &mut self,
        target: &AtspiElement,
        frame: &AtspiElement,
    ) -> Result<Option<Facts>, AdapterError>;
    /// The X11 client of the window and the stacking order, or `None` when the
    /// session is not X11, no manager answers, the window maps to no single
    /// client, or the read ran out of time.
    fn windowing(&mut self, frame: &FrameFacts) -> Option<Windowing>;
}

fn standing(toolkit: &ToolkitId) -> Standing {
    Standing {
        calibrated: true,
        descends: trusts_descent(toolkit),
        same_window: trusts_same_window_landing(toolkit),
    }
}

fn pre_guard(facts: &Facts, point: (i32, i32)) -> bool {
    if reports_global_position(&facts.toolkit) {
        admits(facts.screen, point)
    } else {
        aimable(facts.window)
    }
}

fn intercepted_by_window(frame: Extents, name: Option<String>) -> HitTestResult {
    HitTestResult::InterceptedBy {
        role: Some("window".to_owned()),
        name,
        bounds: Some(frame.into()),
    }
}

fn intercepted_by_lander(lander: Option<Described>, origin: (i32, i32)) -> HitTestResult {
    let lander = lander.unwrap_or_default();
    HitTestResult::InterceptedBy {
        role: lander.role,
        name: lander.name,
        bounds: lander
            .window_extents
            .and_then(|extents| to_pointer_frame(origin, extents))
            .map(Into::into),
    }
}

fn timed_out_or_unknown(deadline: Deadline) -> Result<HitTestResult, AdapterError> {
    if deadline.is_expired() {
        Err(deadline.timeout_error())
    } else {
        Ok(HitTestResult::Unknown)
    }
}

/// The hit test over `world`: both legs and the table. Every unreadable thing
/// is `Unknown`; only a spent deadline or a failed bus read that timed out is
/// an error.
pub(crate) fn hit_test_via<W: HitWorld>(
    world: &mut W,
    target: &AtspiElement,
    frame: &AtspiElement,
    point: (i32, i32),
    deadline: Deadline,
) -> Result<HitTestResult, AdapterError> {
    let Some(facts) = world.facts(target, frame)? else {
        return timed_out_or_unknown(deadline);
    };
    if facts.minimized || !pre_guard(&facts, point) {
        return Ok(HitTestResult::Unknown);
    }
    let Some(windowing) = world.windowing(&facts.frame) else {
        return timed_out_or_unknown(deadline);
    };
    let Some(own) = windowing
        .snapshot
        .stack
        .iter()
        .find(|window| window.id == windowing.client)
    else {
        return Ok(HitTestResult::Unknown);
    };
    if own.flags.hidden || !own.flags.mapped {
        return Ok(HitTestResult::Unknown);
    }
    let shadow = if reports_global_position(&facts.toolkit) {
        (0, 0, 0, 0)
    } else {
        own.shadow
    };
    let origin = windowing
        .coordinate_origin
        .unwrap_or((own.client.x + shadow.0, own.client.y + shadow.1));
    let Some(calibrated) = calibrate(&facts.toolkit, facts.screen, facts.window, Some(origin))
    else {
        return Ok(HitTestResult::Unknown);
    };
    let Some(window_at) = window_point(calibrated.window_origin, point) else {
        return Ok(HitTestResult::Unknown);
    };
    if !admits(calibrated.rect, point) {
        return Ok(HitTestResult::Unknown);
    }
    let stacking = occlusion(&windowing.snapshot, windowing.client, point);
    let standing = standing(&facts.toolkit);
    let (landing, lander) = if needs_landing(standing, stacking) {
        let descent = descend(world, frame, window_at, deadline)?;
        let located = locate(
            world,
            &Probe {
                target,
                frame,
                window_point: window_at,
            },
            descent.as_deref(),
            deadline,
        )?;
        (located.landing, located.lander)
    } else {
        (Landing::NotRead, None)
    };
    Ok(match decide(standing, landing, stacking) {
        Verdict::Reaches => HitTestResult::ReachesTarget,
        Verdict::CoveredBy { window, frame } => {
            let name = cover_name(&windowing.clients, window).or_else(|| {
                override_name(
                    &windowing.snapshot.override_redirect_names,
                    &windowing.clients,
                    window,
                )
            });
            intercepted_by_window(frame, name)
        }
        Verdict::SameWindowLander => intercepted_by_lander(lander, calibrated.window_origin),
        Verdict::Unknown => HitTestResult::Unknown,
    })
}

/// Whether the point reaches the target, is intercepted, or cannot be told.
pub(crate) fn hit_test(
    bus: &SharedBus,
    handle: &NativeHandle,
    point: Point,
    deadline: Deadline,
) -> Result<HitTestResult, AdapterError> {
    let target = atspi_element(handle)?;
    if shell::is_shell_element(target) {
        return shell::hit::hit_test(bus, target, point, deadline);
    }
    let (Some(frame), Some(pixel)) = (target.frame(), pixel(point.x, point.y)) else {
        return Ok(HitTestResult::Unknown);
    };
    if deadline.is_expired() {
        return Err(deadline.timeout_error());
    }
    let connection = bus.get(deadline)?.inner().clone();
    let mut world = live_world::LiveWorld::new(&connection, deadline);
    hit_test_via(&mut world, target, frame, pixel, deadline)
        .map_err(|error| live_world::stopped_aware_for(error, frame))
}

#[cfg(test)]
#[path = "hit_test_tests.rs"]
pub(crate) mod tests;
