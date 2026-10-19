use std::thread::sleep;
use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline};

use super::key_session::KeySession;
use crate::input::key_names::KEYSYM_SHIFT_L;
use crate::input::keysym_step::KeysymStep;

/// What a session sends before its real keys, because the compositor drops
/// the first key of a session and creates the seat keyboard only on input:
/// an optional modifier tap, then an optional wait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LeadPolicy {
    pub(crate) tap: Option<u32>,
    pub(crate) settle: Duration,
}

/// The one place the lead is chosen. The compositor gives the focused window
/// its keyboard about 10 ms after the first key, so the settle lets a global
/// press, which has no frame to wait for, find the window ready.
pub(crate) const LEAD_POLICY: LeadPolicy = LeadPolicy {
    tap: Some(KEYSYM_SHIFT_L),
    settle: Duration::from_millis(100),
};

pub(crate) fn lead_steps(policy: LeadPolicy) -> Vec<KeysymStep> {
    match policy.tap {
        Some(keysym) => vec![
            KeysymStep { keysym, down: true },
            KeysymStep {
                keysym,
                down: false,
            },
        ],
        None => Vec::new(),
    }
}

pub(crate) fn send_lead(
    session: &mut dyn KeySession,
    policy: LeadPolicy,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    for step in lead_steps(policy) {
        session.keysym(step.keysym, step.down, deadline)?;
    }
    if !policy.settle.is_zero() {
        sleep(deadline.remaining_slice(policy.settle)?);
    }
    Ok(())
}

#[cfg(test)]
#[path = "lead_key_tests.rs"]
mod tests;
