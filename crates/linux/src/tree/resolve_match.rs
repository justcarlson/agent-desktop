use agent_desktop_core::{
    IdentityMatch, LocatorEvidence, LocatorField, Rect, RefEntry,
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

/// How far a node's size may drift between a snapshot and a landing on its
/// stored path before the landing is a different element. A toolkit lays a
/// text view out one pixel shorter once its first text lands (Text Editor,
/// 690x464 to 690x463), and the path, the role and the window already say
/// which element it is.
const LAYOUT_JITTER_PX: f64 = 2.0;

const SAME_SIZE_PX: f64 = 0.5;

fn sizes_within(left: (f64, f64), right: (f64, f64), tolerance: f64) -> bool {
    (left.0 - right.0).abs() < tolerance && (left.1 - right.1).abs() < tolerance
}

pub(crate) fn same_size(left: (f64, f64), right: (f64, f64)) -> bool {
    sizes_within(left, right, SAME_SIZE_PX)
}

fn live_size(evidence: &LocatorEvidence) -> Option<(f64, f64)> {
    match &evidence.ref_evidence.bounds {
        LocatorField::Known(rect) => usable_size(rect),
        LocatorField::Absent | LocatorField::Unknown => None,
    }
}

/// The size the ref was taken with, when it carries a rectangle with an area.
/// Position is never read: it changes on every GTK3 and Qt window move and is
/// always zero on GTK4 (L17-19).
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

/// Whether a ref with no identity can be recognised by its size alone: it
/// stored a rectangle with an area and has no identifier or stable text. A
/// zero-area rectangle is shared by every collapsed or virtualised element
/// and identifies nothing.
pub(crate) fn provisional_geometry_candidate(entry: &RefEntry) -> bool {
    stored_size(entry).is_some() && !has_meaningful_identity(entry)
}

/// Whether the candidate's size equals the stored one under the promotion's
/// eligibility rule.
fn geometry_matches_within(entry: &RefEntry, evidence: &LocatorEvidence, tolerance: f64) -> bool {
    provisional_geometry_candidate(entry)
        && stored_size(entry)
            .zip(live_size(evidence))
            .is_some_and(|(stored, live)| sizes_within(stored, live, tolerance))
}

/// Whether the stored size and the live size are both known and differ by
/// more than a layout jitter. An unknown size on either side, or a different
/// position, refutes nothing.
pub(crate) fn size_contradicts(entry: &RefEntry, evidence: &LocatorEvidence) -> bool {
    stored_size(entry)
        .zip(live_size(evidence))
        .is_some_and(|(stored, live)| !sizes_within(stored, live, LAYOUT_JITTER_PX))
}

/// Whether nothing any tier could verify a live node against is stored: no
/// identifier, no stable text and no rectangle with an area.
pub(crate) fn entry_is_unverifiable(entry: &RefEntry) -> bool {
    !has_meaningful_identity(entry) && !provisional_geometry_candidate(entry)
}

/// Decides one node's admission from the stored ref and the node's evidence:
/// the role gate, the composed identity rule and the size promotion. An
/// unreadable role or identity leaves the region unread and is never a
/// refutation. A ref that can only be recognised by its size is refuted by a
/// node whose size was read and differs: it is not unread, since nothing more
/// could be learned about that node.
pub(crate) fn admit_node(entry: &RefEntry, evidence: &LocatorEvidence) -> Admission {
    admit_node_within(entry, evidence, SAME_SIZE_PX)
}

fn admit_node_within(entry: &RefEntry, evidence: &LocatorEvidence, tolerance: f64) -> Admission {
    match evidence.role.known() {
        None if evidence.role.is_unknown() => return Admission::Unread,
        Some(role) if role == &entry.identity.role => {}
        _ => return Admission::Reject,
    }
    match identity_verdict(entry, evidence) {
        IdentityMatch::Match => Admission::Collect,
        IdentityMatch::Unknown if geometry_matches_within(entry, evidence, tolerance) => {
            Admission::Collect
        }
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
/// still answers to the stored identity and has not changed size. It is not a
/// claim that no twin exists elsewhere.
pub(crate) fn accepts_landing(entry: &RefEntry, evidence: &LocatorEvidence) -> bool {
    !size_contradicts(entry, evidence)
        && admit_node_within(entry, evidence, LAYOUT_JITTER_PX) == Admission::Collect
}

fn no_text(field: &LocatorField<String>) -> bool {
    match field {
        LocatorField::Known(text) => text.trim().is_empty(),
        LocatorField::Absent => true,
        LocatorField::Unknown => false,
    }
}

/// Whether nothing the node answers could tell it from any other node of its
/// role: no name or description, no identifier, and no extents with an area.
/// An unreadable name, description or identifier is not an absence, so it
/// withholds the claim. An unreadable extents read is treated like an absent
/// one: a session with no display geometry to report cannot prove a node
/// gained an area, and the role and the stored path are the evidence that
/// remains.
fn landing_is_identityless(evidence: &LocatorEvidence) -> bool {
    no_text(&evidence.name)
        && no_text(&evidence.description)
        && evidence.identifiers.is_complete()
        && evidence.identifiers.preferred_identifier().is_none()
        && live_size(evidence).is_none()
}

/// Decides the node the stored path landed on for a ref that carries no
/// identity to check it against. The path is the only evidence, so the claim
/// is the narrowest one: the node there has the stored role, does not
/// contradict a stored size, and is itself as bare as the ref, so a node that
/// has since gained a name, an identifier or an area is a different element.
pub(crate) fn accepts_identityless_landing(entry: &RefEntry, evidence: &LocatorEvidence) -> bool {
    evidence.role.known() == Some(&entry.identity.role)
        && !size_contradicts(entry, evidence)
        && landing_is_identityless(evidence)
}

#[cfg(test)]
#[path = "resolve_match_tests.rs"]
mod tests;
