//! Pointer input on the bound Hyprland instance through
//! `zwlr_virtual_pointer_v1`, and the one path every headed Hyprland
//! mutation takes to hide the agent's cursors around its events.

pub(crate) mod bound_geometry;
pub(crate) mod cursor_guard;
pub(crate) mod focus_gate;
pub(crate) mod hypr_pointer;
pub(crate) mod hypr_route;
pub(crate) mod live_pointer_sink;
pub(crate) mod native_receiver;
pub(crate) mod output_map;
pub(crate) mod output_point;
pub(crate) mod pointer_cancel;
pub(crate) mod pointer_capability;
pub(crate) mod pointer_devices;
pub(crate) mod pointer_event;
pub(crate) mod pointer_run;
pub(crate) mod pointer_sink;
pub(crate) mod pointer_state;
pub(crate) mod ref_pointer;
pub(crate) mod relative_motion;
pub(crate) mod run_plans;
pub(crate) mod show_on_drop;
pub(crate) mod step_event;
pub(crate) mod wheel_axis;

#[cfg(test)]
pub(crate) mod fake_pointer_sink;
