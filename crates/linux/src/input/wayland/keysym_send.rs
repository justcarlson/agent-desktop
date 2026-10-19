use std::thread::sleep;
use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics};
use serde_json::json;

use super::kept_session;
use super::key_session::KeySession;
use super::layout::refuse_unreachable;
use super::lead_key::{LEAD_POLICY, LeadPolicy, send_lead};
use super::remote_desktop::{RemoteDesktop, semantic_action_refusal};
use crate::input::delivery::{Delivery, Gate};
use crate::input::keysym_step::KeysymStep;

const RECOVERY_BUDGET_MS: u64 = 1_000;
const DRAIN: Duration = Duration::from_millis(150);

fn ensure_budget(deadline: Deadline) -> Result<(), AdapterError> {
    if deadline.is_expired() {
        Err(deadline.timeout_error())
    } else {
        Ok(())
    }
}

fn with_progress(error: AdapterError, delivered_events: u32) -> AdapterError {
    if delivered_events > 0 {
        return error
            .with_details(json!({
                "physical_delivery_started": true,
                "delivered_events": delivered_events,
            }))
            .with_disposition(DeliverySemantics::uncertain());
    }
    if error.disposition != DeliverySemantics::Unknown {
        return error;
    }
    error
        .with_details(json!({ "physical_delivery_started": false }))
        .with_disposition(DeliverySemantics::not_delivered())
}

fn recover(session: &mut dyn KeySession, held: &[u32]) {
    let Ok(deadline) = Deadline::detached_after(RECOVERY_BUDGET_MS) else {
        return;
    };
    for keysym in held.iter().rev() {
        let _ = session.keysym(*keysym, false, deadline);
    }
}

fn send_each(
    session: &mut dyn KeySession,
    keys: &[KeysymStep],
    already_delivered: u32,
    deadline: Deadline,
) -> Result<u32, AdapterError> {
    let mut held: Vec<u32> = Vec::new();
    let mut delivered = 0_u32;
    for key in keys {
        if let Err(error) =
            ensure_budget(deadline).and_then(|()| session.keysym(key.keysym, key.down, deadline))
        {
            recover(session, &held);
            return Err(with_progress(error, already_delivered + delivered));
        }
        delivered += 1;
        if key.down {
            held.push(key.keysym);
        } else if let Some(at) = held.iter().rposition(|keysym| *keysym == key.keysym) {
            held.remove(at);
        }
    }
    Ok(delivered)
}

fn drive(
    session: &mut dyn KeySession,
    lead: LeadPolicy,
    chunks: &[&[KeysymStep]],
    gate: Gate<'_>,
    deadline: Deadline,
) -> Result<u32, AdapterError> {
    ensure_budget(deadline).map_err(|error| with_progress(error, 0))?;
    if !session.resumed() {
        session
            .start(deadline)
            .and_then(|()| send_lead(session, lead, deadline))
            .map_err(|error| with_progress(error, 0))?;
    }
    let mut delivered = 0_u32;
    for chunk in chunks {
        gate().map_err(|error| with_progress_kept(error, delivered))?;
        delivered += send_each(session, chunk, delivered, deadline)?;
    }
    Ok(delivered)
}

fn with_progress_kept(error: AdapterError, delivered: u32) -> AdapterError {
    if delivered == 0 {
        return error;
    }
    with_progress(error, delivered)
}

#[cfg(test)]
pub(crate) fn run(
    session: &mut dyn KeySession,
    lead: LeadPolicy,
    keys: &[KeysymStep],
    gate: Gate<'_>,
    deadline: Deadline,
) -> Result<Delivery, AdapterError> {
    run_chunks(session, lead, &[keys], gate, deadline)
}

pub(crate) fn run_chunks(
    session: &mut dyn KeySession,
    lead: LeadPolicy,
    chunks: &[&[KeysymStep]],
    gate: Gate<'_>,
    deadline: Deadline,
) -> Result<Delivery, AdapterError> {
    if !session.resumed() {
        session
            .create(deadline)
            .map_err(|error| with_progress(error, 0))?;
    }
    let outcome = drive(session, lead, chunks, gate, deadline);
    if outcome.as_ref().is_ok_and(|delivered| *delivered > 0) {
        sleep(deadline.remaining().min(DRAIN));
    }
    let kept = outcome.is_ok() && session.retain();
    if !kept && let Ok(recovery) = Deadline::detached_after(RECOVERY_BUDGET_MS) {
        let _ = session.stop(recovery);
    }
    outcome.map(|delivered_events| Delivery { delivered_events })
}

pub(crate) fn send_chunks(
    chunks: &[Vec<KeysymStep>],
    gate: Gate<'_>,
    deadline: Deadline,
) -> Result<Delivery, AdapterError> {
    for chunk in chunks {
        refuse_unreachable(chunk)?;
    }
    let chunks: Vec<&[KeysymStep]> = chunks
        .iter()
        .map(Vec::as_slice)
        .filter(|chunk| !chunk.is_empty())
        .collect();
    if chunks.is_empty() {
        return Ok(Delivery {
            delivered_events: 0,
        });
    }
    let kept = kept_session::take();
    let reused = kept.is_some();
    let mut session = match kept {
        Some(session) => session,
        None => RemoteDesktop::open(deadline)?,
    };
    let mut outcome = run_chunks(&mut session, LEAD_POLICY, &chunks, gate, deadline);
    if reused && outcome.as_ref().is_err_and(sent_nothing) {
        session = RemoteDesktop::open(deadline)?;
        outcome = run_chunks(&mut session, LEAD_POLICY, &chunks, gate, deadline);
    }
    if outcome.is_ok() && session.resumed() {
        kept_session::keep(session);
    }
    outcome
}

fn sent_nothing(error: &AdapterError) -> bool {
    error.disposition == DeliverySemantics::not_delivered()
}

#[cfg(test)]
pub(crate) fn send_keys(
    keys: &[KeysymStep],
    gate: Gate<'_>,
    deadline: Deadline,
) -> Result<Delivery, AdapterError> {
    send_chunks(&[keys.to_vec()], gate, deadline)
}

pub(crate) fn pointer_refusal(what: &'static str) -> AdapterError {
    semantic_action_refusal(format!(
        "{what} is not available through the compositor on Wayland"
    ))
}

#[cfg(test)]
#[path = "keysym_send_tests.rs"]
mod tests;
