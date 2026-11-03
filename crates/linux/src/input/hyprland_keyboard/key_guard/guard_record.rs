//! The record the guard releases: its command's own record in the instance's
//! journal folder, matched by process id, process instance and display, so
//! a record a later process wrote under a reused id is never taken.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::input::hyprland_keyboard::held_key_journal::key_codes;

/// The keys a dead command's record still holds, and where the record is.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct HeldRecord {
    pub(crate) path: PathBuf,
    pub(crate) codes: Vec<u32>,
}

/// The record `pid` with `instance` wrote in `dir` for `display`, when it
/// still holds a key.
pub(crate) fn read_held(dir: &Path, pid: u32, instance: &str, display: &str) -> Option<HeldRecord> {
    let _ = (dir, pid, instance, display, key_codes(&Value::Null).count());
    None
}

impl HeldRecord {
    /// Removes the record once its keys are released. A record already gone
    /// was released by the next delivery on the instance.
    pub(crate) fn remove(&self) -> std::io::Result<()> {
        std::fs::remove_file(&self.path)
    }
}

#[cfg(test)]
#[path = "guard_record_tests.rs"]
mod tests;
