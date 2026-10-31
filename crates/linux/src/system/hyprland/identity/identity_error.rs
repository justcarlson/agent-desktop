use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode};
use serde_json::{Value, json};

use super::identity_refusal::IdentityRefusal;
use crate::system::focus::focus_cause::{Cause, refusal};

/// The error for a window that could not be bound to one toplevel. Nothing
/// was sent, so every one is safe to retry after a fresh `list-windows`.
pub(crate) fn identity_error(identity: IdentityRefusal) -> AdapterError {
    match identity {
        IdentityRefusal::ProcessChanged => {
            AdapterError::stale_ref_because("The window belongs to a different process instance")
        }
        IdentityRefusal::OtherCompositor => {
            not_found("The window id was issued by a compositor that no longer runs this session")
        }
        IdentityRefusal::ToplevelGone => {
            not_found("The compositor no longer has the toplevel this window id names")
        }
        IdentityRefusal::NoCandidate => refusal(Cause::WindowNotMapped),
        IdentityRefusal::Ambiguous(candidates) => ambiguous(&candidates),
    }
}

fn not_found(message: &str) -> AdapterError {
    AdapterError::new(ErrorCode::WindowNotFound, message)
        .with_suggestion("Run 'list-windows' for the current window ids, then retry")
        .with_disposition(DeliverySemantics::not_delivered())
}

fn ambiguous(candidates: &[(u64, String)]) -> AdapterError {
    let named: Vec<Value> = candidates
        .iter()
        .map(|(address, workspace)| {
            json!({"address": format!("{address:#x}"), "workspace": workspace})
        })
        .collect();
    let base = refusal(Cause::AmbiguousWindow);
    let mut details = base.details.clone().unwrap_or_else(|| json!({}));
    if let Some(fields) = details.as_object_mut() {
        fields.insert("candidates".into(), Value::Array(named));
    }
    base.with_details(details)
}

#[cfg(test)]
#[path = "identity_error_tests.rs"]
mod tests;
