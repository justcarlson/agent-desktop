use agent_desktop_core::{
    BoundsSpace, IdentityMatch, LocatorEvidence, Rect, RefEntry,
    ref_identity::{
        has_meaningful_identity, has_stable_text_identity, identity_match, stable_text_match,
    },
};

/// What a search does with one node it has read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Admission {
    /// The node answers to the stored evidence and counts as a candidate.
    Collect,
    /// The node's evidence could not settle the question, so this region of
    /// the tree was not read conclusively and a negative verdict is withheld.
    Unread,
    /// The node is refuted or its role rules it out.
    Reject,
}

/// The width and height of a rectangle that has an area. A zero or negative
/// side is no size: GTK4 containers often report a zero axis, and `-1` is how
/// a toolkit says it has no extents.
pub(crate) fn usable_size(rect: &Rect) -> Option<(f64, f64)> {
    (rect.width > 0.0 && rect.height > 0.0).then_some((rect.width, rect.height))
}

pub(crate) fn stored_size(entry: &RefEntry) -> Option<(f64, f64)> {
    entry.geometry.bounds.as_ref().and_then(usable_size)
}

/// The identity verdict. An identifier match alone would settle an
/// identifier-bearing ref, but an identifier can be shared (Qt list rows share
/// their list's, L3-9) or reused by a different element, so when the ref also
/// carries stable text the text must agree: an identifier match with a text
/// mismatch is a refutation, and an unreadable text is never a mismatch.
pub(crate) fn identity_verdict(entry: &RefEntry, evidence: &LocatorEvidence) -> IdentityMatch {
    let by_identifier = identity_match(
        entry,
        &evidence.name,
        &evidence.value,
        &evidence.description,
        &evidence.identifiers,
    );
    if entry.identity.native_id.is_some()
        && by_identifier == IdentityMatch::Match
        && has_stable_text_identity(entry)
    {
        return stable_text_match(
            entry,
            &evidence.name,
            &evidence.value,
            &evidence.description,
        );
    }
    by_identifier
}

/// Whether a ref with no identity carries a usable rectangle: it
/// stored a rectangle with an area and has no identifier or stable text. A
/// zero-area rectangle is shared by every collapsed or virtualised element
/// and identifies nothing.
pub(crate) fn provisional_geometry_candidate(entry: &RefEntry) -> bool {
    stored_size(entry).is_some() && !has_meaningful_identity(entry)
}

/// Whether geometry alone proves the same rectangle in its recorded frame.
fn geometry_matches(entry: &RefEntry, evidence: &LocatorEvidence) -> bool {
    provisional_geometry_candidate(entry) && rectangles_match(entry, evidence) == Some(true)
}

pub(crate) fn stored_space(entry: &RefEntry) -> BoundsSpace {
    entry.geometry.bounds_space.unwrap_or_default()
}

fn live_rect(entry: &RefEntry, evidence: &LocatorEvidence) -> Option<Rect> {
    match stored_space(entry) {
        BoundsSpace::Window => evidence.ref_evidence.window_bounds.known().copied(),
        BoundsSpace::Screen => evidence.ref_evidence.bounds.known().copied(),
    }
}

pub(crate) fn rectangles_equal(left: &Rect, right: &Rect) -> bool {
    left == right
}

fn rectangles_match(entry: &RefEntry, evidence: &LocatorEvidence) -> Option<bool> {
    entry
        .geometry
        .bounds
        .as_ref()
        .filter(|rect| usable_size(rect).is_some())
        .zip(live_rect(entry, evidence).filter(|rect| usable_size(rect).is_some()))
        .map(|(stored, live)| rectangles_equal(stored, &live))
}

/// Whether the stored and live evidence describe unequal rectangles.
/// Both position and size must agree in the
/// recorded frame. An unknown rectangle on either side refutes nothing.
pub(crate) fn geometry_contradicts(entry: &RefEntry, evidence: &LocatorEvidence) -> bool {
    rectangles_match(entry, evidence) == Some(false)
}

/// Whether nothing any tier could verify a live node against is stored: no
/// identifier, no stable text and no rectangle with an area.
pub(crate) fn entry_is_unverifiable(entry: &RefEntry) -> bool {
    !has_meaningful_identity(entry) && !provisional_geometry_candidate(entry)
}

/// Decides one node's admission from the stored ref and the node's evidence:
/// the role gate, the composed identity rule and the whole rectangle. An
/// unreadable role or identity leaves the region unread and is never a
/// refutation. A ref that can only be recognised by geometry is refuted by a
/// node whose rectangle was read and differs: it is not unread, since nothing more
/// could be learned about that node.
pub(crate) fn admit_node(entry: &RefEntry, evidence: &LocatorEvidence) -> Admission {
    if entry.identity.retained_object.is_some() && geometry_contradicts(entry, evidence) {
        return Admission::Reject;
    }
    match evidence.role.known() {
        None if evidence.role.is_unknown() => return Admission::Unread,
        Some(role) if role == &entry.identity.role => {}
        _ => return Admission::Reject,
    }
    match identity_verdict(entry, evidence) {
        IdentityMatch::Match => Admission::Collect,
        IdentityMatch::Unknown if geometry_matches(entry, evidence) => Admission::Collect,
        IdentityMatch::Unknown
            if provisional_geometry_candidate(entry)
                && !evidence.ref_evidence.bounds.is_unknown() =>
        {
            Admission::Reject
        }
        IdentityMatch::Unknown => Admission::Unread,
        IdentityMatch::NoMatch => Admission::Reject,
    }
}

/// Decides the node the stored path landed on, from its own evidence alone.
/// The claim is positional: the ref named this place and the element there
/// still answers to the stored identity and recorded rectangle. It is not a
/// claim that no twin exists elsewhere.
pub(crate) fn accepts_landing(entry: &RefEntry, evidence: &LocatorEvidence) -> bool {
    !geometry_contradicts(entry, evidence) && admit_node(entry, evidence) == Admission::Collect
}

#[cfg(test)]
#[path = "resolve_match_tests.rs"]
mod tests;
