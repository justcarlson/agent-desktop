//! The record the guard releases: its command's own record in the instance's
//! journal folder, matched by process id, process instance and display, so
//! a record a later process wrote under a reused id is never taken. The guard
//! claims the record before it reads it, so a delivery's recovery that
//! claimed it first leaves the guard nothing to send.

use std::path::Path;

use agent_desktop_core::Deadline;

use crate::input::hyprland_keyboard::held_key_journal::key_codes;
use crate::input::hyprland_keyboard::record_claim::{RecordClaim, claim_record};

/// The keys a dead command's record still holds, under the guard's claim.
#[derive(Debug)]
pub(crate) struct HeldRecord {
    claim: RecordClaim,
    pub(crate) codes: Vec<u32>,
}

/// Claims the record `pid` with `instance` wrote in `dir` for `display`, when
/// it still holds a key. A record another release claimed past `deadline`, or
/// already removed, gives nothing.
pub(crate) fn claim_held(
    dir: &Path,
    pid: u32,
    instance: &str,
    display: &str,
    deadline: Deadline,
) -> Option<HeldRecord> {
    let claim = claim_record(&dir.join(format!("{pid}.json")), deadline).ok()??;
    let value = claim.value().ok()?;
    let owned = value["pid"].as_u64() == Some(u64::from(pid))
        && !instance.is_empty()
        && value["instance"].as_str() == Some(instance)
        && value["display"].as_str() == Some(display);
    if !owned {
        return None;
    }
    let codes: Vec<u32> = key_codes(&value).collect();
    (!codes.is_empty()).then_some(HeldRecord { claim, codes })
}

impl HeldRecord {
    pub(crate) fn path(&self) -> &Path {
        self.claim.path()
    }

    /// Removes the record once its keys are released, then gives up the
    /// claim.
    pub(crate) fn remove(self) -> std::io::Result<()> {
        self.claim.remove()
    }
}

#[cfg(test)]
#[path = "guard_record_tests.rs"]
mod tests;
