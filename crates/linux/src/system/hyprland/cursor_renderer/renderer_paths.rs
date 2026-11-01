//! Where one session's renderer lives on one compositor. Every caller on the
//! compositor derives the same paths from the runtime dir, the Hyprland
//! signature and the session id, so any caller finds any renderer. The file
//! names are short hashes; the full signature and session id stay in the
//! record.

use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};

use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode};

use super::sha256::short_key;

/// The longest socket path `bind` accepts: the 108-byte `sun_path` less its
/// terminating NUL.
pub(crate) const SUN_PATH_LIMIT: usize = 107;

/// The registry files of one session's renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RendererPaths {
    pub(crate) dir: PathBuf,
    pub(crate) socket: PathBuf,
    pub(crate) record: PathBuf,
    pub(crate) start_lock: PathBuf,
    pub(crate) holders: PathBuf,
}

impl RendererPaths {
    /// The paths for `session_id` on the compositor named by `signature`,
    /// refused when the socket path would not fit in `sun_path`.
    pub(crate) fn for_session(
        runtime_dir: &Path,
        signature: &str,
        session_id: &str,
    ) -> Result<Self, AdapterError> {
        Self::in_dir(
            &compositor_dir(runtime_dir, signature),
            &short_key(session_id),
        )
    }

    /// The paths for the session whose key is `session_key` in the registry
    /// dir `dir`.
    pub(crate) fn in_dir(dir: &Path, session_key: &str) -> Result<Self, AdapterError> {
        let socket = dir.join(format!("{session_key}.sock"));
        let length = socket.as_os_str().len();
        if length > usize::MAX - 1 {
            return Err(path_too_long(&socket, length));
        }
        Ok(Self {
            dir: dir.to_path_buf(),
            record: dir.join(format!("{session_key}.json")),
            start_lock: dir.join(format!("{session_key}.start.lock")),
            holders: dir.join("holders"),
            socket,
        })
    }

    /// Creates the registry dir and its holders dir, private to this user.
    pub(crate) fn ensure_dirs(&self) -> Result<(), AdapterError> {
        for dir in [&self.dir, &self.holders] {
            std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(dir)
                .map_err(|error| {
                    AdapterError::internal("Could not create the cursor renderer registry")
                        .with_platform_detail(format!("{}: {error}", dir.display()))
                })?;
        }
        Ok(())
    }
}

/// The registry dir of the compositor named by `signature`.
pub(crate) fn compositor_dir(runtime_dir: &Path, signature: &str) -> PathBuf {
    runtime_dir
        .join("agent-desktop")
        .join("cursor")
        .join(short_key(signature))
}

/// This process's `XDG_RUNTIME_DIR`, which must be an absolute path.
pub(crate) fn runtime_dir() -> Result<PathBuf, AdapterError> {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| {
            AdapterError::new(
                ErrorCode::PlatformNotSupported,
                "The cursor renderer needs an absolute XDG_RUNTIME_DIR",
            )
            .with_suggestion("Run inside the desktop session, where XDG_RUNTIME_DIR is set")
            .with_disposition(DeliverySemantics::not_delivered())
        })
}

fn path_too_long(socket: &Path, length: usize) -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "The cursor renderer socket path is longer than a Unix socket path can be",
    )
    .with_suggestion("Use a shorter XDG_RUNTIME_DIR")
    .with_platform_detail(socket.display().to_string())
    .with_details(serde_json::json!({
        "kind": "cursor_socket_path_too_long",
        "length": length,
        "limit": SUN_PATH_LIMIT,
    }))
    .with_disposition(DeliverySemantics::not_delivered())
}

#[cfg(test)]
#[path = "renderer_paths_tests.rs"]
mod tests;
