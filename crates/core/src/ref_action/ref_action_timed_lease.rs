//! An interaction lease that remembers when it was acquired, so a coordinate pointer command reports
//! `details.auto_wait.lease_contention_count` and `lease_hold_ms` with the meaning the ref-action path
//! gives them (`ref_action_poll_state.rs`): the contention the lease itself counted while it queued, and
//! the milliseconds from acquisition to release.

use crate::{AdapterError, Deadline, InteractionLease, adapter::PlatformAdapter};
use serde_json::{Value, json};
use std::ops::Deref;
use std::time::Instant;

pub(crate) struct TimedLease {
    lease: InteractionLease,
    acquired: Instant,
}

impl TimedLease {
    pub(crate) fn acquire(
        adapter: &dyn PlatformAdapter,
        deadline: Deadline,
    ) -> Result<Self, AdapterError> {
        let lease = adapter.acquire_interaction_lease(deadline)?;
        Ok(Self {
            lease,
            acquired: Instant::now(),
        })
    }

    /// Releases the lease and records its contention and hold time in the response's
    /// `details.auto_wait`, keeping every detail the adapter reported.
    pub(crate) fn release_into(self, response: &mut Value) {
        let lease_hold_ms = u64::try_from(self.acquired.elapsed().as_millis()).unwrap_or(u64::MAX);
        let metrics = json!({
            "lease_contention_count": self.lease.contention_count(),
            "lease_hold_ms": lease_hold_ms,
        });
        drop(self.lease);
        attach_metrics(response, metrics);
    }
}

impl Deref for TimedLease {
    type Target = InteractionLease;

    fn deref(&self) -> &InteractionLease {
        &self.lease
    }
}

fn attach_metrics(response: &mut Value, metrics: Value) {
    let Some(body) = response.as_object_mut() else {
        return;
    };
    let details = match body.remove("details") {
        None | Some(Value::Null) => json!({ "auto_wait": metrics }),
        Some(Value::Object(mut fields)) => {
            fields.insert("auto_wait".into(), metrics);
            Value::Object(fields)
        }
        Some(other) => json!({ "action_details": other, "auto_wait": metrics }),
    };
    body.insert("details".into(), details);
}

#[cfg(test)]
#[path = "ref_action_timed_lease_tests.rs"]
mod tests;
