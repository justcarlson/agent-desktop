//! The check after the hide: every `agent-desktop-cursor` layer the
//! compositor lists must belong to a renderer that acknowledged this
//! holder. A renderer that started after the hide went out gets its `hide`
//! now; any other such layer is a foreign surface and refuses.

use super::cursor_surface::CURSOR_NAMESPACE;
use super::hide_refusal::HideRefusal;
use crate::system::hyprland::receiver::layer_info::LayerInfo;

/// Checks `layers` against the acknowledged renderer pids `acked`.
/// `late_hide(pid)` hides a renderer found after the first round and says
/// whether `pid` is one; a pid it acknowledges joins `acked`.
pub(crate) fn audit_cursor_layers(
    layers: &[LayerInfo],
    acked: &mut Vec<u32>,
    late_hide: &mut dyn FnMut(u32) -> Result<bool, HideRefusal>,
) -> Result<(), HideRefusal> {
    for layer in layers
        .iter()
        .filter(|layer| layer.namespace == CURSOR_NAMESPACE)
    {
        let unknown = || HideRefusal::UnknownOverlay {
            pid: layer.pid,
            output: layer.output.clone(),
        };
        let pid = layer.pid.ok_or_else(unknown)?;
        if acked.contains(&pid) {
            continue;
        }
        if !late_hide(pid)? {
            return Err(unknown());
        }
        acked.push(pid);
    }
    Ok(())
}

#[cfg(test)]
#[path = "layer_audit_tests.rs"]
mod tests;
