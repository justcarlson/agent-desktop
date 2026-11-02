//! Hyprland 0.56.2 moves a mapped layer surface toward a new margin with its
//! `layersIn` animation, so the cursor is drawn between its old and new
//! point after the frame callback of its arrival has fired (probe row
//! L43-6). Before it maps a surface, the renderer asks its compositor for a
//! `no_anim` layer rule on the cursor namespace, so a margin change is drawn
//! where it is committed. The rule is named, so a later renderer replaces it
//! instead of adding another. A compositor without a Lua config refuses the
//! request and keeps animating.

use agent_desktop_core::Deadline;

use super::cursor_surface::CURSOR_NAMESPACE;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::ipc::bound_request::bound_request;

const RULE_MS: u64 = 500;

/// The IPC command that turns off layer animation for cursor surfaces.
pub(crate) fn no_anim_command() -> String {
    format!(
        "eval hl.layer_rule({{ name = \"{CURSOR_NAMESPACE}-no-anim\", \
         match = {{ namespace = \"^{CURSOR_NAMESPACE}$\" }}, no_anim = true }})"
    )
}

/// Asks the bound compositor not to animate cursor surfaces, and returns
/// whether it accepted the rule.
pub(crate) fn suppress_layer_animation(binding: &HyprlandBinding) -> bool {
    Deadline::detached_after(RULE_MS)
        .and_then(|deadline| bound_request(binding, &no_anim_command(), deadline))
        .is_ok_and(|reply| reply.trim_ascii() == b"ok")
}

#[cfg(test)]
#[path = "layer_animation_tests.rs"]
mod tests;
