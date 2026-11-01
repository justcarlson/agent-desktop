//! One hide holder on disk: `<registry>/holders/<nonce>.json`, written
//! through a private temporary file and a rename, mode 0600, holding
//! `{"pid","start_time","nonce","expiry"}` with `expiry` in Unix
//! milliseconds. A holder is live while its file exists, its process runs
//! with the recorded start time, and its expiry has not passed. Every
//! renderer on the compositor draws nothing while any holder is live.

use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use agent_desktop_core::{AdapterError, ErrorCode};
use serde_json::{Value, json};

use crate::system::process::proc_stat::read_stat;

#[cfg_attr(not(test), allow(dead_code))]
const MAX_NONCE_BYTES: usize = 64;

/// The fields of one holder file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HolderFile {
    pub(crate) pid: u32,
    pub(crate) start_time: u64,
    pub(crate) nonce: String,
    pub(crate) expiry: u64,
}

impl HolderFile {
    /// The file this holder lives in under `holders`, refused for a nonce
    /// that is not a plain name.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn path_in(holders: &Path, nonce: &str) -> Result<PathBuf, AdapterError> {
        let plain = !nonce.is_empty()
            && nonce.len() <= MAX_NONCE_BYTES
            && nonce
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
        if !plain {
            return Err(AdapterError::new(
                ErrorCode::InvalidArgs,
                "A cursor hide holder nonce must be 1 to 64 letters, digits, '-' or '_'",
            ));
        }
        Ok(holders.join(format!("{nonce}.json")))
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn to_json(&self) -> Value {
        json!({
            "pid": self.pid,
            "start_time": self.start_time,
            "nonce": self.nonce,
            "expiry": self.expiry,
        })
    }

    pub(crate) fn parse(bytes: &[u8]) -> Option<Self> {
        let value: Value = serde_json::from_slice(bytes).ok()?;
        Some(Self {
            pid: u32::try_from(value.get("pid")?.as_u64()?).ok()?,
            start_time: value.get("start_time")?.as_u64()?,
            nonce: value.get("nonce")?.as_str()?.to_owned(),
            expiry: value.get("expiry")?.as_u64()?,
        })
    }

    /// Writes this holder into `holders` atomically and privately.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn write_in(&self, holders: &Path) -> Result<PathBuf, AdapterError> {
        let path = Self::path_in(holders, &self.nonce)?;
        let staging = holders.join(format!(".{}.tmp", self.nonce));
        let written = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&staging)
            .and_then(|mut file| {
                file.write_all(self.to_json().to_string().as_bytes())?;
                file.sync_all()
            })
            .and_then(|()| std::fs::rename(&staging, &path));
        written.map_err(|error| {
            let _ = std::fs::remove_file(&staging);
            AdapterError::internal("Could not write the cursor hide holder")
                .with_platform_detail(format!("{}: {error}", path.display()))
        })?;
        Ok(path)
    }

    /// Whether this holder still suppresses drawing at `now_ms`.
    pub(crate) fn is_live(&self, now_ms: u64) -> bool {
        now_ms < self.expiry
            && read_stat(self.pid)
                .ok()
                .flatten()
                .is_some_and(|stat| stat.start_time == self.start_time)
    }
}

/// Milliseconds since the Unix epoch.
pub(crate) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
#[path = "holder_file_tests.rs"]
mod tests;
