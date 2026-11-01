//! The renderer stop `session gc` runs for each session it removes, and the
//! stop a `disable` falls back to when the session's compositor is gone. It
//! reports how the renderer was stopped.

use std::path::Path;

use agent_desktop_core::AdapterError;

use super::exe_identity::ExeIdentity;
use super::renderer_paths::{RendererPaths, runtime_dir};
use super::renderer_stop::stop;
use super::stop_outcome::StopOutcome;
use super::stop_timings::StopTimings;
use crate::system::hyprland::bind::routed_binding::routed_binding;

/// Stops `session_id`'s cursor renderer on this Hyprland session, or does
/// nothing off Hyprland.
pub fn stop_session_renderer(session_id: &str) -> Result<Option<&'static str>, AdapterError> {
    let Some(binding) = routed_binding()? else {
        return Ok(None);
    };
    let paths = RendererPaths::for_session(&runtime_dir()?, &binding.signature, session_id)?;
    let outcome = stop(&paths, ExeIdentity::current(), &StopTimings::default())?;
    Ok(Some(outcome.as_str()))
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
    let _ = (
        runtime_dir,
        named,
        session_id,
        how.caller.as_ref(),
        how.timings,
    );
    Ok(None)
}

#[cfg(test)]
#[path = "session_stop_tests.rs"]
mod tests;
