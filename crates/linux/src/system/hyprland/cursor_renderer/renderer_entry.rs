//! The renderer's entry point. An `agent-desktop` process started with the
//! renderer marker reads its startup control from standard input, binds the
//! Hyprland instance its environment names, and serves until it stops. A
//! marker for another protocol version is refused.

use std::io::Read;

use agent_desktop_core::{AdapterError, CursorOverlayControl, ErrorCode};

use super::renderer_paths::{RendererPaths, runtime_dir};
use super::renderer_process::RendererProcess;
use super::renderer_spawn::{RENDERER_ENV, RENDERER_PROTOCOL};
use super::renderer_target::RendererTarget;
use super::renderer_wire::MAX_REQUEST_BYTES;
use crate::system::hyprland::bind::session::session_binding;

/// Runs the renderer when this process was started as one.
pub(crate) fn entry_from_env() -> Option<Result<(), AdapterError>> {
    let marker = std::env::var_os(RENDERER_ENV)?;
    if marker != RENDERER_PROTOCOL {
        return Some(Err(AdapterError::new(
            ErrorCode::InvalidArgs,
            format!("This build serves cursor renderer protocol {RENDERER_PROTOCOL} only"),
        )));
    }
    Some(run())
}

fn run() -> Result<(), AdapterError> {
    let first = read_first_control()?;
    let binding = session_binding()
        .map_err(|refusal| refusal.to_error())?
        .ok_or_else(|| AdapterError::not_supported("cursor renderer outside a Hyprland session"))?;
    let target = RendererTarget {
        binding,
        runtime_dir: runtime_dir()?,
    };
    let paths = RendererPaths::for_session(
        &target.runtime_dir,
        &target.binding.signature,
        first.session_id(),
    )?;
    RendererProcess::start(paths, &target, &first)?.serve()
}

fn read_first_control() -> Result<CursorOverlayControl, AdapterError> {
    let mut payload = Vec::new();
    std::io::stdin()
        .take(MAX_REQUEST_BYTES)
        .read_to_end(&mut payload)
        .map_err(|error| {
            AdapterError::internal("The cursor renderer could not read its startup control")
                .with_platform_detail(error.to_string())
        })?;
    let control: CursorOverlayControl = serde_json::from_slice(&payload).map_err(|error| {
        AdapterError::new(
            ErrorCode::InvalidArgs,
            "The cursor renderer startup control is invalid",
        )
        .with_platform_detail(error.to_string())
    })?;
    control.validate()?;
    Ok(control)
}
