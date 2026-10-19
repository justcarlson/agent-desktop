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

#[expect(dead_code, reason = "stub the owning lane replaces")]
pub(crate) fn focus_is_indeterminate(_candidates: &[usize], _decided: Option<usize>) -> bool {
    false
}

#[cfg(test)]
#[path = "focus_tests.rs"]
mod tests;
