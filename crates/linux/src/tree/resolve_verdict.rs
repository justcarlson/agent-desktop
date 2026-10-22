use agent_desktop_core::{BoundsSpace, Rect};

use super::resolve_match::{rectangles_equal, usable_size};

/// What the candidates of a search mean for the ref.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// The candidate at this index is the element.
    Resolved(usize),
    /// Every region was read and nothing answers to the stored evidence.
    Stale,
    /// Two or more candidates answer and nothing separates them.
    Ambiguous,
    /// A region could not be read, so the answer is withheld.
    Incomplete,
}

/// The one candidate whose whole rectangle matches in its recorded frame.
/// It only ever picks among several candidates and never refutes one; a ref
/// with no stored size cannot be separated this way and stays ambiguous. A
/// candidate whose extents were not read is not a size that differs: it could
/// be the element, so while any candidate is unread nothing is picked.
pub(crate) fn select_by_stored_geometry(
    candidates: &[Option<Rect>],
    spaces: &[BoundsSpace],
    stored: Option<&Rect>,
    stored_space: BoundsSpace,
) -> Option<usize> {
    if stored.and_then(usable_size).is_none() || candidates.iter().any(Option::is_none) {
        return None;
    }
    if spaces.len() != candidates.len() || spaces.iter().any(|space| *space != stored_space) {
        return None;
    }
    let stored = stored?;
    let mut matching = candidates.iter().enumerate().filter_map(|(index, rect)| {
        let rect = rect.as_ref()?;
        let same = usable_size(rect).is_some() && rectangles_equal(rect, stored);
        same.then_some(index)
    });
    let first = matching.next()?;
    matching.next().is_none().then_some(first)
}

/// Classifies the candidates a search collected. Two or more candidates with
/// no usable stored size are ambiguous whatever else is unread, because no
/// unread region can make them anything else. Below that, an unread region
/// withholds every verdict, so a lone match found while another region could
/// not be read is retried and not resolved.
pub(crate) fn classify(
    candidates: &[Option<Rect>],
    spaces: &[BoundsSpace],
    stored: Option<&Rect>,
    stored_space: BoundsSpace,
    unread_region: bool,
) -> Verdict {
    let separable = stored.and_then(usable_size).is_some();
    if candidates.len() > 1 && !separable {
        return Verdict::Ambiguous;
    }
    let extents_unread = candidates.len() > 1 && candidates.iter().any(Option::is_none);
    if unread_region || extents_unread {
        return Verdict::Incomplete;
    }
    match candidates.len() {
        0 => Verdict::Stale,
        1 => Verdict::Resolved(0),
        _ => select_by_stored_geometry(candidates, spaces, stored, stored_space)
            .map_or(Verdict::Ambiguous, Verdict::Resolved),
    }
}

#[cfg(test)]
#[path = "resolve_verdict_tests.rs"]
mod tests;
