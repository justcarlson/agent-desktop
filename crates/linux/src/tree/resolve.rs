use agent_desktop_core::ref_identity::has_meaningful_identity;
use agent_desktop_core::resolve_errors::identity_unknown_error;
use agent_desktop_core::{AdapterError, Deadline, ErrorCode, LocatorEvidence, Rect, RefEntry};

use super::AtspiElement;
use super::path_descent::{Landing, Level, descend};
use super::resolve_errors::{
    ambiguous, is_incomplete, no_candidate, settle_gone, stopped_aware, unverifiable_entry,
};
use super::resolve_match::{accepts_landing, entry_is_unverifiable};
use super::resolve_search::{Candidate, Searched};
use super::resolve_verdict::{Verdict, classify};
use super::walker_expand::UNKNOWN_PID;

/// What a resolution reads. The bus implements it; a test substitutes an
/// in-memory tree and counts the reads.
pub(crate) trait Source {
    /// The verified window frame the ref was taken in, carrying its ancestry.
    fn open_window(&mut self, entry: &RefEntry) -> Result<AtspiElement, AdapterError>;
    /// The children of one node, in the order a walk records them.
    fn level(&mut self, element: &AtspiElement) -> Result<Level<AtspiElement>, AdapterError>;
    /// The evidence of one node, read as the snapshot walk reads it.
    fn evidence(&mut self, element: &AtspiElement) -> Result<LocatorEvidence, AdapterError>;
    /// Every node under the frame, evaluated against the stored evidence.
    fn search(&mut self, frame: &AtspiElement, entry: &RefEntry) -> Result<Searched, AdapterError>;
    /// The listed name of the application when `/proc` shows it stopped.
    fn stopped_app(&mut self, pid: u32) -> Option<String>;
}

/// Whether the stored path is scoped to the window root rather than to a
/// drill-down ancestor.
fn window_rooted(entry: &RefEntry) -> bool {
    entry.scope.root_ref.is_none() || entry.scope.path_is_absolute
}

fn path_tier_applies(entry: &RefEntry) -> bool {
    window_rooted(entry) && !entry.scope.path.is_empty()
}

/// The element the stored path lands on when it still answers to the stored
/// identity and has not changed size, and whether the descent left a region
/// unread. Declining is never a verdict: the search decides. A landing that
/// was destroyed is settled gone, as the anchor path settles it, so it
/// declines without leaving a region unread; any other failed read does.
fn path_tier<S: Source>(
    source: &mut S,
    frame: &AtspiElement,
    entry: &RefEntry,
    deadline: Deadline,
) -> Result<(Option<AtspiElement>, bool), AdapterError> {
    let landing = descend(frame.clone(), &entry.scope.path, deadline, |element| {
        source.level(element)
    })?;
    let Landing { element, unread } = landing;
    let Some(element) = element.filter(|element| element.pid() != UNKNOWN_PID) else {
        return Ok((None, unread));
    };
    match source.evidence(&element) {
        Ok(evidence) if accepts_landing(entry, &evidence) => Ok((Some(element), unread)),
        Ok(_) => Ok((None, unread)),
        Err(error) if matches!(error.code, ErrorCode::AppNotFound | ErrorCode::Timeout) => {
            Err(error)
        }
        Err(error) if error.code == ErrorCode::ElementNotFound => Ok((None, false)),
        Err(_) => Ok((None, true)),
    }
}

/// The one candidate that sits as deep as the stored path is long, when
/// several answer to the stored evidence. A control that exposes a twin of
/// itself as its own child (a Files row's cell holds a cell of the same name
/// and size) puts the twins at different depths, and a sibling removed from
/// the list shifts every index but no depth. It only ever picks among several
/// candidates and never refutes one.
fn sole_at_stored_depth<'a>(
    candidates: &'a [Candidate],
    entry: &RefEntry,
) -> Option<&'a Candidate> {
    if candidates.len() < 2 {
        return None;
    }
    let depth = entry.scope.path.len();
    let mut at_depth = candidates
        .iter()
        .filter(|candidate| candidate.depth == depth);
    let first = at_depth.next()?;
    at_depth.next().is_none().then_some(first)
}

/// The verified window frame, when the ref stored an empty path and the frame
/// answers to the stored evidence. The gate has proved the process and the
/// window, and exactly one node sits at depth zero, so no twin can exist
/// elsewhere in the tree and an unread region below it (a background tab's
/// document lists no children) cannot change the answer.
fn frame_candidate<'a>(candidates: &'a [Candidate], entry: &RefEntry) -> Option<&'a Candidate> {
    if !window_rooted(entry) || !entry.scope.path.is_empty() {
        return None;
    }
    candidates.iter().find(|candidate| candidate.depth == 0)
}

fn attempt<S: Source>(
    source: &mut S,
    entry: &RefEntry,
    deadline: Deadline,
) -> Result<AtspiElement, AdapterError> {
    if entry_is_unverifiable(entry) {
        return Err(unverifiable_entry());
    }
    if deadline.is_expired() {
        return Err(deadline.timeout_error());
    }
    let frame = source.open_window(entry)?;
    let path_unread = if path_tier_applies(entry) {
        let (landed, unread) = path_tier(source, &frame, entry, deadline)?;
        if let Some(element) = landed
            && entry.identity.retained_object.is_some()
        {
            return Ok(element);
        }
        unread
    } else {
        false
    };
    let searched = source.search(&frame, entry)?;
    let rects: Vec<Option<Rect>> = searched
        .candidates
        .iter()
        .map(|candidate| candidate.bounds)
        .collect();
    let spaces: Vec<agent_desktop_core::BoundsSpace> = searched
        .candidates
        .iter()
        .map(|candidate| candidate.bounds_space)
        .collect();
    if let Some(frame) = frame_candidate(&searched.candidates, entry) {
        return Ok(frame.element.clone());
    }
    let unread = path_unread || searched.unread_region;
    if window_rooted(entry)
        && !unread
        && let Some(candidate) = sole_at_stored_depth(&searched.candidates, entry)
    {
        return Ok(candidate.element.clone());
    }
    match classify(
        &rects,
        &spaces,
        entry
            .geometry
            .bounds
            .as_ref()
            .filter(|_| has_meaningful_identity(entry)),
        super::resolve_match::stored_space(entry),
        unread,
    ) {
        Verdict::Resolved(index) => searched
            .candidates
            .into_iter()
            .nth(index)
            .map(|candidate| candidate.element)
            .ok_or_else(no_candidate),
        Verdict::Stale => Err(no_candidate()),
        Verdict::Ambiguous => Err(ambiguous(searched.candidates.len())),
        Verdict::Incomplete => Err(identity_unknown_error(entry)),
    }
}

/// Resolves a stored ref to the live element it was taken from, or refuses.
/// A ref with no identity evidence is refused before any read: a child index
/// cannot distinguish an unnamed node from a replacement or reordered sibling.
/// The window gate proves the process and window are the stored ones; the
/// stored child-index path can identify a retained object. Every other ref
/// searches the bounded window tree: a positional landing cannot prove that
/// no matching sibling exists.
/// An application that left, a destroyed object and a timeout are told apart
/// at this boundary, and a timeout on a stopped process is `APP_UNRESPONSIVE`.
pub(crate) fn resolve<S: Source>(
    source: &mut S,
    entry: &RefEntry,
    deadline: Deadline,
) -> Result<AtspiElement, AdapterError> {
    if let Some(name) = source.stopped_app(entry.process.pid.get()) {
        return Err(AdapterError::app_unresponsive(&name));
    }
    let outcome = attempt(source, entry, deadline);
    settle_outcome(source, entry, outcome)
}

/// Maps the end of one attempt to the answer the caller gets: an application
/// that left and a destroyed object are settled refusals, and a timeout on a
/// stopped process is `APP_UNRESPONSIVE`.
pub(super) fn settle_outcome<S: Source>(
    source: &mut S,
    entry: &RefEntry,
    outcome: Result<AtspiElement, AdapterError>,
) -> Result<AtspiElement, AdapterError> {
    outcome.map_err(|error| {
        let error = settle_gone(error);
        if error.code == ErrorCode::Timeout || is_incomplete(&error) {
            let stopped = source.stopped_app(entry.process.pid.get());
            stopped_aware(error, stopped.as_deref())
        } else {
            error
        }
    })
}

#[cfg(test)]
#[path = "resolve_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "resolve_depth_tests.rs"]
mod depth_tests;

#[cfg(test)]
#[path = "resolve_refusal_tests.rs"]
mod refusal_tests;

#[cfg(test)]
#[path = "resolve_web_tests.rs"]
mod web_tests;

#[cfg(test)]
#[path = "resolve_shift_tests.rs"]
mod shift_tests;
