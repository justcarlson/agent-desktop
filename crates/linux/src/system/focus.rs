use super::x11::X11Evidence;

/// The one focused window, or none. `candidates` are the indices of the
/// listed windows that carry the `ACTIVE` state. A toolkit can set that state
/// on a window no window manager made active, so with a window manager's
/// answer the candidate must agree with it; without one a single candidate
/// is believed and several are not. Where the sources do not agree, nothing
/// is claimed.
pub(crate) fn decide_focus(candidates: &[usize], evidence: X11Evidence) -> Option<usize> {
    match (candidates, evidence) {
        ([], _) => None,
        ([only], X11Evidence::Unavailable) => Some(*only),
        (_, X11Evidence::Frame(named)) => candidates.contains(&named).then_some(named),
        _ => None,
    }
}

/// Whether the sources named an `ACTIVE` window and still could not agree on
/// one. `decided` is the answer of the same `decide_focus` call, so the
/// branching lives in one place. Without a candidate there is nothing to
/// decide, so no focus is a true answer.
pub(crate) fn focus_is_indeterminate(candidates: &[usize], decided: Option<usize>) -> bool {
    !candidates.is_empty() && decided.is_none()
}

#[cfg(test)]
#[path = "focus_tests.rs"]
mod tests;
