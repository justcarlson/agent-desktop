//! Receiver evidence for ref pointer input on the bound Hyprland instance:
//! the point lies inside the target's shown toplevel, and no top or overlay
//! layer surface and no floating, pinned or fullscreen window holds it.
//! Hyprland reports no input region for any surface, so a surface whose
//! rectangle holds the point counts as a cover.

pub(crate) mod check_receiver;
pub(crate) mod deny_all_overlays;
pub(crate) mod layer_info;
pub(crate) mod overlay_filter;
pub(crate) mod own_cursor_filter;
pub(crate) mod receiver_hit;
pub(crate) mod receiver_read;
pub(crate) mod receiver_refusal;
pub(crate) mod stack;
pub(crate) mod stack_client;
pub(crate) mod stack_parse;
pub(crate) mod stacking;

#[cfg(test)]
pub(crate) mod fake_stack_ipc;
