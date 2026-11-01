//! The Hyprland route of `update_cursor_overlay`. A control goes to the
//! session's renderer over its socket; when none answers, the caller starts
//! one under the start lock and sends the control again. `disable` stops the
//! renderer through the teardown and keeps the outcome for the command's
//! `details.cursor_renderer`. A transient hide or show never starts one.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use agent_desktop_core::{
    AdapterError, CURSOR_ARRIVAL_TIMEOUT_MS, CursorOverlayControl, DeliverySemantics, ErrorCode,
};
use serde_json::json;

use super::exe_identity::ExeIdentity;
use super::live_launcher::LiveLauncher;
use super::renderer_launcher::RendererLauncher;
use super::renderer_paths::{RendererPaths, runtime_dir};
use super::renderer_request::RendererRequest;
use super::renderer_start::{StartRequest, ensure_running};
use super::renderer_stop::stop;
use super::renderer_target::RendererTarget;
use super::renderer_wire::exchange;
use super::stop_outcome::StopOutcome;
use super::stop_timings::StopTimings;
use crate::system::hyprland::bind::routed_binding::routed_binding;

const CONTROL_MS: u64 = 4_000;
const START_MS: u64 = 4_000;

static OUTCOMES: Mutex<Option<HashMap<String, &'static str>>> = Mutex::new(None);

/// Applies `control` on a Wayland session: through the session's renderer
/// on Hyprland, refused as not rendered elsewhere.
pub(crate) fn update_wayland(control: &CursorOverlayControl) -> Result<(), AdapterError> {
    control.validate()?;
    let target = match routed_binding()? {
        Some(binding) => Some(RendererTarget {
            binding,
            runtime_dir: runtime_dir()?,
        }),
        None => None,
    };
    update_routed(control, target, &LiveLauncher)
}

/// Applies `control` through the renderer for `target`, or refuses as not
/// rendered when the session is not on Hyprland.
pub(crate) fn update_routed(
    control: &CursorOverlayControl,
    target: Option<RendererTarget>,
    launcher: &dyn RendererLauncher,
) -> Result<(), AdapterError> {
    control.validate()?;
    let _ = (target, launcher);
    return Err(AdapterError::not_supported("cursor overlay on Wayland")
        .with_details(json!({"rendered": false, "cause": "wayland_session"})));
    #[allow(unreachable_code)]
    let Some(target) = target else {
        return Err(AdapterError::not_supported("cursor overlay on Wayland")
            .with_details(json!({"rendered": false, "cause": "wayland_session"})));
    };
    let paths = RendererPaths::for_session(
        &target.runtime_dir,
        &target.binding.signature,
        control.session_id(),
    )?;
    if control.is_disable() {
        return disable(control.session_id(), &paths);
    }
    let request = RendererRequest::control(control.clone());
    if exchange(&paths.socket, &request, reply_deadline(control))?.is_some() {
        return Ok(());
    }
    if control.is_transient() {
        return Ok(());
    }
    let start = StartRequest {
        paths: &paths,
        target: &target,
        first: control,
        deadline: Instant::now() + Duration::from_millis(START_MS),
    };
    ensure_running(&start, launcher, ExeIdentity::current())?;
    match exchange(&paths.socket, &request, reply_deadline(control))? {
        Some(_) => Ok(()),
        None => Err(AdapterError::new(
            ErrorCode::ActionFailed,
            "The cursor renderer stopped before it answered",
        )
        .with_details(json!({"kind": "cursor_renderer_gone"}))
        .with_disposition(DeliverySemantics::not_delivered())),
    }
}

fn reply_deadline(control: &CursorOverlayControl) -> Instant {
    let budget = if control.instruction().is_some() {
        CURSOR_ARRIVAL_TIMEOUT_MS
    } else {
        CONTROL_MS
    };
    Instant::now() + Duration::from_millis(budget)
}

fn disable(session_id: &str, paths: &RendererPaths) -> Result<(), AdapterError> {
    let outcome = stop(paths, ExeIdentity::current(), &StopTimings::default())?;
    OUTCOMES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get_or_insert_with(HashMap::new)
        .insert(session_id.to_owned(), outcome.as_str());
    if outcome == StopOutcome::ExitUnconfirmed {
        return Err(AdapterError::new(
            ErrorCode::ActionFailed,
            "The cursor renderer did not exit after SIGKILL",
        )
        .with_details(json!({"kind": "cursor_renderer_exit_unconfirmed"}))
        .with_disposition(DeliverySemantics::not_delivered()));
    }
    Ok(())
}

/// The teardown outcome of the last `disable` for `session_id`, taken once.
pub fn take_outcome(session_id: &str) -> Option<&'static str> {
    OUTCOMES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_mut()?
        .remove(session_id)
}

#[cfg(test)]
#[path = "hypr_cursor_tests.rs"]
mod tests;
