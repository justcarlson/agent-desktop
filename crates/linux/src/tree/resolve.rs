use agent_desktop_core::resolve_errors::identity_unknown_error;
use agent_desktop_core::{AdapterError, Deadline, ErrorCode, LocatorEvidence, Rect, RefEntry};

use super::AtspiElement;
use super::path_descent::{Landing, Level, descend};
use super::resolve_errors::{
    ambiguous, no_candidate, settle_gone, stopped_aware, unverifiable_entry,
};
use super::resolve_match::{accepts_landing, entry_is_unverifiable};
use super::resolve_search::Searched;
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
/// unread. Declining is never a verdict: the search decides.
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
        Err(_) => Ok((None, true)),
    }
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
        if let Some(element) = landed {
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
    let unread = path_unread || searched.unread_region;
    match classify(&rects, entry.geometry.bounds.as_ref(), unread) {
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
/// Four tiers: a ref with nothing to verify a node against is refused before
/// any read; the window gate proves the process and window are the stored
/// ones; the stored child-index path is followed and its landing accepted when
/// it still answers to the stored identity and size; otherwise a bounded
/// search of the window's tree decides among zero, one and several candidates.
/// An application that left, a destroyed object and a timeout are told apart
/// at this boundary, and a timeout on a stopped process is `APP_UNRESPONSIVE`.
pub(crate) fn resolve<S: Source>(
    source: &mut S,
    entry: &RefEntry,
    deadline: Deadline,
) -> Result<AtspiElement, AdapterError> {
    attempt(source, entry, deadline).map_err(|error| {
        let error = settle_gone(error);
        if error.code == ErrorCode::Timeout {
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
#[path = "resolve_refusal_tests.rs"]
mod refusal_tests;

#[cfg(test)]
#[path = "resolve_web_tests.rs"]
mod web_tests;
