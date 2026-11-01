//! The renderer stop `session gc` runs for each session it removes, and the
//! stop a `disable` falls back to when the session's compositor is gone. It
//! reports how the renderer was stopped.

use std::path::{Path, PathBuf};

use agent_desktop_core::AdapterError;

use super::exe_identity::ExeIdentity;
use super::renderer_paths::{RendererPaths, compositor_dir, runtime_dir};
use super::renderer_stop::stop;
use super::sha256::short_key;
use super::stop_outcome::StopOutcome;
use super::stop_timings::StopTimings;
use crate::system::hyprland::bind::bind_request::plain_signature;
use crate::system::hyprland::bind::routed_binding::routed_binding;

/// Stops `session_id`'s cursor renderer on this Hyprland session. Without a
/// live Hyprland route it stops whatever the registry holds for the session,
/// and does nothing when the registry holds nothing and no instance is named.
pub fn stop_session_renderer(session_id: &str) -> Result<Option<&'static str>, AdapterError> {
    let unrouted = match routed_binding() {
        Ok(Some(binding)) => {
            let paths =
                RendererPaths::for_session(&runtime_dir()?, &binding.signature, session_id)?;
            let outcome = stop(&paths, ExeIdentity::current(), &StopTimings::default())?;
            return Ok(Some(outcome.as_str()));
        }
        Ok(None) => Ok(None),
        Err(refusal) => Err(refusal),
    };
    match (stop_without_route(session_id)?, unrouted) {
        (Some(outcome), _) => Ok(Some(outcome.as_str())),
        (None, unrouted) => unrouted,
    }
}

/// Stops `session_id`'s renderer from the registry alone, for a session
/// whose compositor no route reaches. The instance named by
/// `HYPRLAND_INSTANCE_SIGNATURE` counts even when its registry dir is gone.
pub(crate) fn stop_without_route(session_id: &str) -> Result<Option<StopOutcome>, AdapterError> {
    let Ok(runtime_dir) = runtime_dir() else {
        return Ok(None);
    };
    let named = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
        .ok()
        .filter(|signature| plain_signature(signature));
    let how = UnroutedStop {
        caller: ExeIdentity::current(),
        timings: &StopTimings::default(),
    };
    stop_unrouted(&runtime_dir, named.as_deref(), session_id, &how)
}

/// Who asks for an unrouted stop, and how long it waits.
pub(crate) struct UnroutedStop<'a> {
    pub(crate) caller: Option<ExeIdentity>,
    pub(crate) timings: &'a StopTimings,
}

/// Stops `session_id`'s renderer in every registry dir under `runtime_dir`
/// that holds its record, and in the dir of the `named` signature, without
/// reaching a compositor. `None` when no dir applies.
pub(crate) fn stop_unrouted(
    runtime_dir: &Path,
    named: Option<&str>,
    session_id: &str,
    how: &UnroutedStop<'_>,
) -> Result<Option<StopOutcome>, AdapterError> {
    let key = short_key(session_id);
    let mut dirs: Vec<PathBuf> = named
        .map(|signature| compositor_dir(runtime_dir, signature))
        .into_iter()
        .collect();
    let registry = runtime_dir.join("agent-desktop").join("cursor");
    for entry in std::fs::read_dir(&registry).into_iter().flatten().flatten() {
        let dir = entry.path();
        let holds_record = dir.join(format!("{key}.json")).symlink_metadata().is_ok();
        if holds_record && !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    let mut found: Option<StopOutcome> = None;
    for dir in dirs {
        let outcome = stop(&RendererPaths::in_dir(&dir, &key)?, how.caller, how.timings)?;
        found = Some(found.map_or(outcome, |earlier| earlier.graver(outcome)));
    }
    Ok(found)
}

#[cfg(test)]
#[path = "session_stop_tests.rs"]
mod tests;
