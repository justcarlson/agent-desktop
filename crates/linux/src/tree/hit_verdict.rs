use super::Extents;
use super::hit_occlusion::Occlusion;

/// Where the in-window descent ended, relative to the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Landing {
    /// On the target or one of its descendants.
    OnTarget,
    /// On a node the target sits under.
    Ancestor,
    /// On a node outside the target's ancestry. `contained` is whether the
    /// landing node's own window extents have positive area and hold the
    /// point.
    Elsewhere { contained: bool },
    /// On a node whose ancestry could not be proved but whose rectangle lies
    /// inside the target's and holds the point.
    Inside,
    /// Cut short, null at the frame, or an ancestry that could not be proved.
    Unresolved,
    /// Not read because the stacking order already decided.
    NotRead,
}

/// What is known of the target's toolkit and geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Standing {
    /// The toolkit's window-coordinate descent is trusted to land on a target.
    pub(crate) descends: bool,
    /// The toolkit's landings on unrelated nodes are trusted as interception.
    pub(crate) same_window: bool,
    /// The toolkit's point query stops at a container of the target without
    /// reaching it, so a landing on an ancestor refutes nothing once sibling
    /// separation is proved.
    pub(crate) shallow: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    Reaches,
    CoveredBy { window: u32, frame: Extents },
    SameWindowLander,
    Unknown,
}

/// Whether the descent needs to run: only when the stacking order has not
/// already named a cover and the geometry and toolkit can be relied on.
pub(crate) fn needs_landing(standing: Standing, occlusion: Occlusion) -> bool {
    standing.descends && occlusion == Occlusion::Clear
}

/// The two legs and the table. Geometry that was not proved makes every input
/// `Unknown`; a managed cover above the target intercepts whatever the
/// descent landed on; the rest need the stacking order clear.
pub(crate) fn decide(standing: Standing, landing: Landing, occlusion: Occlusion) -> Verdict {
    if !standing.descends {
        return Verdict::Unknown;
    }
    if let Occlusion::CoveredBy { window, frame } = occlusion {
        return Verdict::CoveredBy { window, frame };
    }
    if occlusion != Occlusion::Clear {
        return Verdict::Unknown;
    }
    match landing {
        Landing::OnTarget => Verdict::Reaches,
        Landing::Ancestor | Landing::Inside if standing.shallow => Verdict::Reaches,
        Landing::Elsewhere { contained: true } if standing.same_window => Verdict::SameWindowLander,
        Landing::Elsewhere { .. }
        | Landing::Ancestor
        | Landing::Inside
        | Landing::Unresolved
        | Landing::NotRead => Verdict::Unknown,
    }
}
