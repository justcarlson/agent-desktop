//! The renderer stop `session gc` runs for each session it removes. It acts
//! only on the Hyprland route and reports how the renderer was stopped.

use agent_desktop_core::AdapterError;

use super::exe_identity::ExeIdentity;
use super::renderer_paths::{RendererPaths, runtime_dir};
use super::renderer_stop::stop;
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
