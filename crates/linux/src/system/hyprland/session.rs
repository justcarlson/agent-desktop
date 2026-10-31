//! The binding of this process, decided at most once while it holds. A bound
//! instance is reused while its compositor process lives with the same start
//! time and the environment still asks for the same instance, so a long-lived
//! library caller rebinds after a compositor restart. Any other outcome is
//! reused only briefly, which keeps a silent endpoint from costing the whole
//! bind budget on every route resolve within one command.

use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use agent_desktop_core::{AdapterError, Deadline};

use super::bind_refusal::BindRefusal;
use super::bind_request::BindRequest;
use super::hyprland_bind::bind;
use super::hyprland_binding::HyprlandBinding;
use super::live_observe::LiveObserve;
use super::observe::Observe;
use super::session_decision::SessionDecision;
use crate::system::display::wayland_shell::WaylandShell;

/// How long one binding decision may read, across every socket it tries.
pub(crate) const BIND_BUDGET_MS: u64 = 1_000;

/// How long an outcome without a binding is reused.
pub(crate) const UNBOUND_REUSE: Duration = Duration::from_secs(2);

static DECIDED: Mutex<Option<SessionDecision>> = Mutex::new(None);

/// This process's binding, deciding it when no reusable decision exists.
pub(crate) fn session_binding() -> Result<Option<HyprlandBinding>, BindRefusal> {
    let request = BindRequest::from_process();
    let mut decided = DECIDED.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(decision) = decided.as_ref()
        && reusable(decision, &request, &LiveObserve, Instant::now())
    {
        return decision.outcome.clone();
    }
    let outcome = match Deadline::after(BIND_BUDGET_MS) {
        Ok(deadline) => bind(&request, &LiveObserve, deadline),
        Err(_) => Ok(None),
    };
    *decided = Some(SessionDecision {
        request,
        outcome: outcome.clone(),
        decided_at: Instant::now(),
    });
    outcome
}

/// Whether an earlier decision still answers for `request` at `now`.
pub(crate) fn reusable(
    decision: &SessionDecision,
    request: &BindRequest,
    observe: &dyn Observe,
    now: Instant,
) -> bool {
    if decision.request != *request {
        return false;
    }
    match &decision.outcome {
        Ok(Some(binding)) => observe.start_time(binding.pid) == Some(binding.start_time),
        _ => now.saturating_duration_since(decision.decided_at) < UNBOUND_REUSE,
    }
}

/// The Wayland shell this session's route uses.
pub(crate) fn session_shell() -> Result<WaylandShell, AdapterError> {
    match session_binding() {
        Ok(Some(binding)) => Ok(WaylandShell::Hyprland(binding)),
        Ok(None) => Ok(WaylandShell::Generic),
        Err(refusal) => Err(refusal.to_error()),
    }
}

/// Whether discovery found a Hyprland session, bound or refused, for a
/// process whose environment names no display server.
pub(crate) fn discovers_a_session() -> bool {
    !matches!(session_binding(), Ok(None))
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
