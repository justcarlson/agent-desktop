//! A caller's own hide holder: written before any renderer is asked to hide,
//! renewed while long work runs, and removed on every way out. Taking one
//! also removes holder files their callers abandoned: a dead caller's, or one
//! expired for longer than any renewal could take.

use std::path::{Path, PathBuf};

use agent_desktop_core::AdapterError;

use super::holder_file::{HolderFile, now_ms};
use super::renderer_owner::fresh_generation;
use crate::system::process::proc_stat::read_stat;

/// The longest a holder suppresses drawing before its caller renews it.
pub(crate) const HOLDER_CAP_MS: u64 = 10_000;

const EXPIRY_GRACE_MS: u64 = 500;

/// One live holder this process wrote.
#[derive(Debug)]
pub(crate) struct HolderLease {
    holders: PathBuf,
    file: HolderFile,
    released: bool,
}

impl HolderLease {
    /// Writes a fresh holder in `holders` that expires at `expiry` (Unix ms).
    pub(crate) fn take(holders: &Path, expiry: u64) -> Result<Self, AdapterError> {
        sweep_abandoned(holders, now_ms());
        let pid = std::process::id();
        let start_time = read_stat(pid)
            .ok()
            .flatten()
            .ok_or_else(|| AdapterError::internal("Could not read this process's start time"))?
            .start_time;
        let nonce = fresh_generation()
            .ok_or_else(|| AdapterError::internal("Could not draw a cursor hide holder nonce"))?;
        let file = HolderFile {
            pid,
            start_time,
            nonce,
            expiry,
        };
        file.write_in(holders)?;
        Ok(Self {
            holders: holders.to_path_buf(),
            file,
            released: false,
        })
    }

    pub(crate) fn nonce(&self) -> &str {
        &self.file.nonce
    }

    pub(crate) fn expiry(&self) -> u64 {
        self.file.expiry
    }

    /// Rewrites the holder with the later `expiry`.
    pub(crate) fn renew(&mut self, expiry: u64) -> Result<(), AdapterError> {
        let mut renewed = self.file.clone();
        renewed.expiry = expiry;
        renewed.write_in(&self.holders)?;
        self.file = renewed;
        Ok(())
    }

    /// Removes the holder file. Later calls do nothing.
    pub(crate) fn release(&mut self) {
        if std::mem::replace(&mut self.released, true) {
            return;
        }
        if let Ok(path) = HolderFile::path_in(&self.holders, &self.file.nonce) {
            let _ = std::fs::remove_file(path);
        }
    }
}

impl Drop for HolderLease {
    fn drop(&mut self) {
        self.release();
    }
}

/// The expiry for work that must end by `work_end` (Unix ms), seen at `now`:
/// a short grace after the work's end, capped at [`HOLDER_CAP_MS`] from now.
pub(crate) fn expiry_for(work_end: u64, now: u64, cap_ms: u64) -> u64 {
    work_end
        .saturating_add(EXPIRY_GRACE_MS)
        .min(now.saturating_add(cap_ms))
}

/// Removes the holder files in `holders` whose process is gone or whose
/// expiry passed more than [`HOLDER_CAP_MS`] ago. A file that does not parse
/// is not a holder and stays.
pub(crate) fn sweep_abandoned(holders: &Path, now: u64) {
    let Ok(entries) = std::fs::read_dir(holders) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let Some(holder) = std::fs::read(&path)
            .ok()
            .and_then(|bytes| HolderFile::parse(&bytes))
        else {
            continue;
        };
        let gone = !holder.is_live(0);
        let long_expired = now > holder.expiry.saturating_add(HOLDER_CAP_MS);
        if gone || long_expired {
            let _ = std::fs::remove_file(&path);
        }
    }
}

#[cfg(test)]
#[path = "holder_lease_tests.rs"]
mod tests;
