use agent_desktop_core::Rect;

use super::resolve_match::{same_size, usable_size};

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

/// The one candidate whose size equals the stored size, when exactly one does.
/// It only ever picks among several candidates and never refutes one; a ref
/// with no stored size cannot be separated this way and stays ambiguous. A
/// candidate whose extents were not read is not a size that differs: it could
/// be the element, so while any candidate is unread nothing is picked.
pub(crate) fn select_by_stored_size(
    candidates: &[Option<Rect>],
    stored: Option<&Rect>,
) -> Option<usize> {
    let stored = stored.and_then(usable_size)?;
    if candidates.iter().any(Option::is_none) {
        return None;
    }
    let mut sole = None;
    let mut matches = 0;
    for (index, rect) in candidates.iter().enumerate() {
        if rect
            .as_ref()
            .and_then(usable_size)
            .is_some_and(|size| same_size(size, stored))
        {
            matches += 1;
            sole = Some(index);
        }
    }
    sole.filter(|_| matches == 1)
}

/// Classifies the candidates a search collected. Two or more candidates with
/// no usable stored size are ambiguous whatever else is unread, because no
/// unread region can make them anything else. Below that, an unread region
/// withholds every verdict, so a lone match found while another region could
/// not be read is retried and not resolved.
pub(crate) fn classify(
    candidates: &[Option<Rect>],
    stored: Option<&Rect>,
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
        _ => {
            select_by_stored_size(candidates, stored).map_or(Verdict::Ambiguous, Verdict::Resolved)
        }
    }
}

#[cfg(test)]
#[path = "resolve_verdict_tests.rs"]
mod tests;
