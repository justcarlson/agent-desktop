//! The record a renderer writes beside its socket once it has bound. It is
//! flat JSON on disk, written atomically and private to this user:
//! `pid`, `start_time`, `exe`, `generation`, `protocol`, `session_id`,
//! `hypr_signature`, `wayland_socket`, `created_at`.

use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use agent_desktop_core::AdapterError;
use serde_json::{Value, json};

use super::renderer_owner::RendererOwner;

/// One renderer's registry record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RendererRecord {
    pub(crate) owner: RendererOwner,
    pub(crate) protocol: String,
    pub(crate) session_id: String,
    pub(crate) hypr_signature: String,
    pub(crate) wayland_socket: PathBuf,
    pub(crate) created_at: u64,
}

impl RendererRecord {
    pub(crate) fn to_json(&self) -> Value {
        json!({
            "pid": self.owner.pid,
            "start_time": self.owner.start_time,
            "exe": self.owner.exe.to_string_lossy(),
            "generation": self.owner.generation,
            "protocol": self.protocol,
            "session_id": self.session_id,
            "hypr_signature": self.hypr_signature,
            "wayland_socket": self.wayland_socket.to_string_lossy(),
            "created_at": self.created_at,
        })
    }

    /// The record `bytes` hold, or `None` when they are not a whole record.
    pub(crate) fn parse(bytes: &[u8]) -> Option<Self> {
        let value: Value = serde_json::from_slice(bytes).ok()?;
        let text = |key: &str| value.get(key)?.as_str().map(str::to_owned);
        let generation = text("generation").filter(|generation| !generation.is_empty())?;
        Some(Self {
            owner: RendererOwner {
                pid: u32::try_from(value.get("pid")?.as_u64()?).ok()?,
                start_time: value.get("start_time")?.as_u64()?,
                exe: PathBuf::from(text("exe")?),
                generation,
            },
            protocol: text("protocol")?,
            session_id: text("session_id")?,
            hypr_signature: text("hypr_signature")?,
            wayland_socket: PathBuf::from(text("wayland_socket")?),
            created_at: value.get("created_at")?.as_u64()?,
        })
    }

    /// Writes the record to `path` through a private temporary file and a
    /// rename, so a reader sees the old record or the whole new one.
    pub(crate) fn write_atomic(&self, path: &Path) -> Result<(), AdapterError> {
        let staging = path.with_extension(format!("{}.tmp", self.owner.generation));
        let written = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&staging)
            .and_then(|mut file| {
                file.write_all(self.to_json().to_string().as_bytes())?;
                file.sync_all()
            })
            .and_then(|()| std::fs::rename(&staging, path));
        written.map_err(|error| {
            let _ = std::fs::remove_file(&staging);
            AdapterError::internal("Could not write the cursor renderer record")
                .with_platform_detail(format!("{}: {error}", path.display()))
        })
    }
}

/// Seconds since the Unix epoch.
pub(crate) fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

#[cfg(test)]
#[path = "renderer_record_tests.rs"]
mod tests;
