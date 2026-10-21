use agent_desktop_core::{AdapterError, Deadline};

use super::hit_guard::admits;
use super::hit_verdict::Landing;
use super::{AtspiElement, Extents};

mod bus_nodes;

pub(crate) use bus_nodes::BusNodes;

/// How many `GetAccessibleAtPoint` levels one descent follows, and how many
/// `Parent` steps one climb takes.
pub(crate) const DEPTH_CAP: usize = 50;

/// What `GetAccessibleAtPoint` answered.
#[derive(Debug, Clone)]
pub(crate) enum Hit {
    Object(AtspiElement),
    Null,
    Failed,
}

/// What a landing node says about itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Described {
    pub(crate) role: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) window_extents: Option<Extents>,
}

/// The reads a descent makes. The bus implements it; a test substitutes
/// in-memory nodes and records every call. `Component.Contains` is not among
/// them: it answers for rectangles, not for what is on top.
pub(crate) trait NodeSource {
    /// `GetAccessibleAtPoint` on `from` in window coordinates.
    fn at_point(&mut self, from: &AtspiElement, point: (i32, i32)) -> Result<Hit, AdapterError>;
    /// The parent of `node` when `Parent` names one whose children list `node`.
    fn verified_parent(
        &mut self,
        node: &AtspiElement,
    ) -> Result<Option<AtspiElement>, AdapterError>;
    fn describe(&mut self, node: &AtspiElement) -> Result<Described, AdapterError>;
}

pub(crate) fn same(a: &AtspiElement, b: &AtspiElement) -> bool {
    a.bus_name() == b.bus_name() && a.path() == b.path()
}

fn ensure_budget(deadline: Deadline) -> Result<(), AdapterError> {
    if deadline.is_expired() {
        Err(deadline.timeout_error())
    } else {
        Ok(())
    }
}

/// Follows point queries from the window. `None` means no verified landing;
/// a later null or a self-hit returns the verified chain.
pub(crate) fn descend<S: NodeSource>(
    source: &mut S,
    frame: &AtspiElement,
    point: (i32, i32),
    deadline: Deadline,
) -> Result<Option<Vec<AtspiElement>>, AdapterError> {
    let mut chain: Vec<AtspiElement> = Vec::new();
    let mut from = frame.clone();
    for _ in 0..DEPTH_CAP {
        ensure_budget(deadline)?;
        match source.at_point(&from, point)? {
            Hit::Failed => return Ok(None),
            Hit::Null => {
                return Ok((!chain.is_empty()).then_some(chain));
            }
            Hit::Object(next) => {
                if same(&next, &from) {
                    if chain.is_empty() {
                        chain.push(next);
                    }
                    return Ok(Some(chain));
                }
                chain.push(next.clone());
                from = next;
            }
        }
    }
    Ok(None)
}

enum Climb {
    Met,
    Passed,
    Unproved,
}

/// Walks verified parents from `start` until `meet` is reached, the window
/// frame is reached, or a step cannot be verified.
fn climb<S: NodeSource>(
    source: &mut S,
    start: &AtspiElement,
    meet: &AtspiElement,
    frame: &AtspiElement,
    deadline: Deadline,
) -> Result<Climb, AdapterError> {
    let mut at = start.clone();
    for _ in 0..DEPTH_CAP {
        ensure_budget(deadline)?;
        let Some(parent) = source.verified_parent(&at)? else {
            return Ok(Climb::Unproved);
        };
        if same(&parent, meet) {
            return Ok(Climb::Met);
        }
        if same(&parent, frame) {
            return Ok(Climb::Passed);
        }
        at = parent;
    }
    Ok(Climb::Unproved)
}

/// The landing the descent reached, relative to the target, with what the
/// landing node says about itself when it lies outside the target's ancestry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Located {
    pub(crate) landing: Landing,
    pub(crate) lander: Option<Described>,
}

fn resolved(landing: Landing) -> Located {
    Located {
        landing,
        lander: None,
    }
}

/// What a descent looked for: the target, the window frame it ran in and the
/// point it probed in window coordinates.
#[derive(Clone, Copy)]
pub(crate) struct Probe<'a> {
    pub(crate) target: &'a AtspiElement,
    pub(crate) frame: &'a AtspiElement,
    pub(crate) window_point: (i32, i32),
}

/// Classifies a verified chain. An outside landing requires both ancestry
/// directions to be verified; any missing edge stays unresolved.
pub(crate) fn locate<S: NodeSource>(
    source: &mut S,
    probe: &Probe,
    chain: Option<&[AtspiElement]>,
    deadline: Deadline,
) -> Result<Located, AdapterError> {
    let Probe {
        target,
        frame,
        window_point,
    } = *probe;
    let Some(chain) = chain else {
        return Ok(resolved(Landing::Unresolved));
    };
    let Some(lander) = chain.last() else {
        return Ok(resolved(Landing::Unresolved));
    };
    if chain.iter().any(|node| same(node, target)) {
        return Ok(resolved(Landing::OnTarget));
    }
    if same(lander, frame) {
        return Ok(resolved(Landing::Ancestor));
    }
    match climb(source, lander, target, frame, deadline)? {
        Climb::Met => return Ok(resolved(Landing::OnTarget)),
        Climb::Unproved => return Ok(resolved(Landing::Unresolved)),
        Climb::Passed => {}
    }
    match climb(source, target, lander, frame, deadline)? {
        Climb::Met => return Ok(resolved(Landing::Ancestor)),
        Climb::Unproved => return Ok(resolved(Landing::Unresolved)),
        Climb::Passed => {}
    }
    let described = source.describe(lander)?;
    let contained = described
        .window_extents
        .is_some_and(|extents| admits(extents, window_point));
    Ok(Located {
        landing: Landing::Elsewhere { contained },
        lander: Some(described),
    })
}

#[cfg(test)]
#[path = "hit_descent_tests.rs"]
pub(crate) mod tests;
