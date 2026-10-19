use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::owner_files;
use crate::system::process_facts::parse_start_time;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OwnerRecord {
    pub(crate) pid: u32,
    pub(crate) start_ticks: u64,
    pub(crate) display: String,
    pub(crate) window: u32,
    pub(crate) nonce: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Outcome {
    Ready(OwnerRecord),
    Failed(String),
}

const FILE_PREFIX: &str = "owner-";
const FILE_SUFFIX: &str = ".json";
const NONCE_LENGTHS: std::ops::RangeInclusive<usize> = 8..=64;

/// The display name that a display shares with and without its default
/// screen suffix, made safe for use in a file name.
pub(crate) fn normalise_display(display: &str) -> String {
    let screen_free = display.strip_suffix(".0").unwrap_or(display);
    screen_free
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub(crate) fn valid_nonce(nonce: &str) -> bool {
    NONCE_LENGTHS.contains(&nonce.len()) && nonce.bytes().all(|b| b.is_ascii_alphanumeric())
}

pub(crate) fn file_name(display: &str, nonce: &str) -> Option<String> {
    valid_nonce(nonce).then(|| {
        format!(
            "{FILE_PREFIX}{}-{nonce}{FILE_SUFFIX}",
            normalise_display(display)
        )
    })
}

pub(crate) fn record_dir() -> Option<PathBuf> {
    agent_desktop_core::session::agent_desktop_dir()
        .ok()
        .map(|root| root.join("clipboard"))
}

impl OwnerRecord {
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        json!({
            "pid": self.pid,
            "start_ticks": self.start_ticks,
            "display": normalise_display(&self.display),
            "window": self.window,
            "nonce": self.nonce,
        })
        .to_string()
        .into_bytes()
    }

    pub(crate) fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let value: Value = serde_json::from_slice(bytes).ok()?;
        Some(Self {
            pid: u32::try_from(value.get("pid")?.as_u64()?).ok()?,
            start_ticks: value.get("start_ticks")?.as_u64()?,
            display: value.get("display")?.as_str()?.to_owned(),
            window: u32::try_from(value.get("window")?.as_u64()?).ok()?,
            nonce: value.get("nonce")?.as_str()?.to_owned(),
        })
    }
}

pub(crate) fn outcome_of(bytes: &[u8]) -> Option<Outcome> {
    if let Some(record) = OwnerRecord::from_bytes(bytes) {
        return Some(Outcome::Ready(record));
    }
    let value: Value = serde_json::from_slice(bytes).ok()?;
    value.get("nonce")?.as_str()?;
    Some(Outcome::Failed(value.get("error")?.as_str()?.to_owned()))
}

pub(crate) fn publish(dir: &Path, record: &OwnerRecord) -> std::io::Result<()> {
    let name = file_name(&record.display, &record.nonce).ok_or_else(invalid_nonce)?;
    owner_files::ensure_private_dir(dir)?;
    owner_files::write_new(dir, &name, &record.to_bytes())
}

pub(crate) fn publish_failure(
    dir: &Path,
    display: &str,
    nonce: &str,
    message: &str,
) -> std::io::Result<()> {
    let name = file_name(display, nonce).ok_or_else(invalid_nonce)?;
    owner_files::ensure_private_dir(dir)?;
    let bytes = json!({ "nonce": nonce, "error": message })
        .to_string()
        .into_bytes();
    owner_files::write_new(dir, &name, &bytes)
}

fn invalid_nonce() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "the record nonce is not valid",
    )
}

/// Removes the owner's own record, and only when the file still carries its
/// nonce: a file someone else put under the name is not ours to delete.
pub(crate) fn remove_own(dir: &Path, record: &OwnerRecord) {
    let Some(name) = file_name(&record.display, &record.nonce) else {
        return;
    };
    let carries_our_nonce = std::fs::read(dir.join(&name))
        .ok()
        .and_then(|bytes| OwnerRecord::from_bytes(&bytes))
        .is_some_and(|stored| stored.nonce == record.nonce);
    if carries_our_nonce {
        let _ = owner_files::remove(dir, &name);
    }
}

pub(crate) fn read_outcome(dir: &Path, display: &str, nonce: &str) -> Option<Outcome> {
    let name = file_name(display, nonce)?;
    outcome_of(&std::fs::read(dir.join(name)).ok()?)
}

pub(crate) fn records_in(dir: &Path, display: &str) -> Vec<OwnerRecord> {
    let wanted = normalise_display(display);
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with(FILE_PREFIX) && name.ends_with(FILE_SUFFIX)
        })
        .filter_map(|entry| std::fs::read(entry.path()).ok())
        .filter_map(|bytes| OwnerRecord::from_bytes(&bytes))
        .filter(|record| record.display == wanted)
        .collect()
}

pub(crate) fn start_ticks_of(pid: u32) -> Option<u64> {
    parse_start_time(&std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?)
}

/// A record is live only while the process it names started when the record
/// says. A different start time means the pid was recycled, and a recycled
/// pid is never signalled.
#[cfg(test)]
pub(crate) fn is_live_with(record: &OwnerRecord, start_of: impl Fn(u32) -> Option<u64>) -> bool {
    record.pid != 0 && start_of(record.pid) == Some(record.start_ticks)
}

#[cfg(test)]
#[path = "owner_record_tests.rs"]
mod tests;
