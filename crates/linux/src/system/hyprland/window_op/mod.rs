//! `move-window`, `resize-window`, `minimize`, `maximize` and `restore` on
//! Hyprland. The window is bound to one compositor toplevel, the request is
//! sent over the bound IPC socket by that toplevel's address, and each
//! effect is proven by an independent `j/clients` read-back. Probe rows
//! S9-1 to S9-35 decide the rules: move and resize act on floating windows
//! only, maximize never takes the user's focus, and minimize parks the
//! window on a special workspace that names the workspace it came from.

pub(crate) mod client_state;
pub(crate) mod config_dialect;
pub(crate) mod hypr_window_op;
pub(crate) mod live_window_world;
pub(crate) mod step_goal;
pub(crate) mod window_command;
pub(crate) mod window_op_entry;
pub(crate) mod window_op_refusal;
pub(crate) mod window_plan;
pub(crate) mod window_step;
pub(crate) mod window_world;
#[cfg(test)]
pub(crate) mod window_world_fake;
