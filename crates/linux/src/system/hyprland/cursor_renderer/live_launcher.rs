//! Starts the real renderer: this `agent-desktop` executable re-run with the
//! renderer marker. A library host is not that executable, so it refuses as
//! not supported and the caller reports `rendered: false`.

use std::io::Write;

use agent_desktop_core::{AdapterError, CursorOverlayControl};

use super::renderer_launcher::RendererLauncher;
use super::renderer_paths::RendererPaths;
use super::renderer_spawn::renderer_command;
use super::renderer_target::RendererTarget;

const RENDERER_EXECUTABLE: &str = "agent-desktop";

/// The launcher for the shipped CLI.
pub(crate) struct LiveLauncher;

impl RendererLauncher for LiveLauncher {
    fn session_active(&self, control: &CursorOverlayControl) -> bool {
        session_active(control.session_id(), control.agent_id())
    }

    fn launch(
        &self,
        target: &RendererTarget,
        _paths: &RendererPaths,
        first: &CursorOverlayControl,
    ) -> Result<(), AdapterError> {
        let executable = std::env::current_exe().map_err(|error| {
            AdapterError::internal("Could not locate the cursor renderer executable")
                .with_platform_detail(error.to_string())
        })?;
        if executable.file_stem().and_then(|name| name.to_str()) != Some(RENDERER_EXECUTABLE) {
            return Err(AdapterError::not_supported(
                "cursor renderer outside the agent-desktop executable",
            )
            .with_details(serde_json::json!({"rendered": false, "cause": "library_host"})));
        }
        let payload = serde_json::to_vec(first).map_err(|error| {
            AdapterError::internal("Could not encode the cursor renderer startup control")
                .with_platform_detail(error.to_string())
        })?;
        let mut child = renderer_command(
            &executable,
            &target.binding,
            &target.runtime_dir,
            std::env::vars_os(),
        )
        .spawn()
        .map_err(|error| {
            AdapterError::internal("Could not start the cursor renderer")
                .with_platform_detail(error.to_string())
        })?;
        let sent = child
            .stdin
            .take()
            .map(|mut stdin| stdin.write_all(&payload));
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        match sent {
            Some(Ok(())) => Ok(()),
            _ => Err(AdapterError::internal(
                "Could not hand the cursor renderer its startup control",
            )),
        }
    }
}

/// Whether `session_id` is live with the cursor enabled for this caller.
pub(crate) fn session_active(session_id: &str, agent_id: Option<&str>) -> bool {
    agent_desktop_core::session::read_manifest(session_id)
        .ok()
        .flatten()
        .is_some_and(|manifest| {
            manifest.ended_at.is_none()
                && manifest.cursor_overlay.is_enabled()
                && !(manifest.cursor_overlay.is_multi_agent() && agent_id.is_none())
        })
}

/// Whether `session_id` still wants a renderer: its manifest exists, the
/// session has not ended, and the cursor is on. An unreadable manifest
/// keeps the renderer, so a transient read error does not end it.
pub(crate) fn session_wants_cursor(session_id: &str) -> bool {
    match agent_desktop_core::session::read_manifest(session_id) {
        Ok(Some(manifest)) => manifest.ended_at.is_none() && manifest.cursor_overlay.is_enabled(),
        Ok(None) => false,
        Err(_) => true,
    }
}
