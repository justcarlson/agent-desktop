//! The cursor hider for the bound Hyprland instance: it hides every agent
//! cursor in the compositor's renderer registry and proves, from the
//! compositor's layer stack, that no other `agent-desktop-cursor` layer is
//! on screen. Without a runtime dir no renderer can run, so only the layer
//! proof remains.

use std::path::{Path, PathBuf};
use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline};

use super::bind::hyprland_binding::HyprlandBinding;
use super::cursor_hider::CursorHider;
use super::cursor_renderer::hide_broadcast::HIDE_DEADLINE_MS;
use super::cursor_renderer::hide_hold::HideHold;
use super::cursor_renderer::hide_refusal::HideRefusal;
use super::cursor_renderer::hold_renewal::RenewTimings;
use super::cursor_renderer::layer_audit::audit_cursor_layers;
use super::cursor_renderer::renderer_paths::compositor_dir;
use super::ipc::bound_request::bound_request;
use super::receiver::layer_info::LayerInfo;
use super::receiver::stack_parse::parse_layers;

const LAYERS_COMMAND: &str = "j/layers";

/// Hides the agent cursors on `binding` while a hold lasts.
pub(crate) struct RegistryCursorHider {
    binding: HyprlandBinding,
    registry: Option<PathBuf>,
    hold: Option<HideHold>,
}

/// The hider for `binding`, whose renderers register under `runtime_dir`.
pub(crate) fn cursor_hider_for(
    binding: &HyprlandBinding,
    runtime_dir: Option<&Path>,
) -> Box<dyn CursorHider> {
    Box::new(RegistryCursorHider {
        binding: binding.clone(),
        registry: runtime_dir.map(|runtime| compositor_dir(runtime, &binding.signature)),
        hold: None,
    })
}

impl CursorHider for RegistryCursorHider {
    fn hide(&mut self, deadline: Deadline) -> Result<(), AdapterError> {
        self.show();
        let binding = &self.binding;
        let mut read = |deadline| read_layers(binding, deadline);
        match &self.registry {
            Some(dir) => {
                self.hold = Some(HideHold::take(
                    dir,
                    deadline,
                    RenewTimings::STANDARD,
                    &mut read,
                )?);
            }
            None => {
                let window = deadline.capped(Duration::from_millis(HIDE_DEADLINE_MS));
                read(window)
                    .map_err(|error| HideRefusal::Failed {
                        cause: "layer_stack",
                        detail: error.message,
                    })
                    .and_then(|layers| {
                        audit_cursor_layers(&layers, &mut Vec::new(), &mut |_| Ok(false))
                    })
                    .map_err(|refusal| refusal.to_error())?;
            }
        }
        Ok(())
    }

    fn show(&mut self) {
        if let Some(mut hold) = self.hold.take() {
            hold.release();
        }
    }
}

fn read_layers(
    binding: &HyprlandBinding,
    deadline: Deadline,
) -> Result<Vec<LayerInfo>, AdapterError> {
    let body = bound_request(binding, LAYERS_COMMAND, deadline)?;
    parse_layers(&body)
        .ok_or_else(|| AdapterError::internal("Hyprland sent a j/layers reply that does not parse"))
}

#[cfg(test)]
#[path = "registry_cursor_hider_tests.rs"]
mod tests;
